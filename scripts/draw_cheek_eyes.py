"""Draw source-reviewed vector eye contours into the existing native face UVs.

Uses the same skin/blush/accessory masks as natural eyelids. Only new cheek
atlases are saved; neutral textures, blend files, mesh data and blink files are
read-only. The editable contour definitions below are the source of the art.
Run in Blender: --background --python-exit-code 1 --python this_file -- [slugs]
"""
import bpy, json, sys, struct, zlib
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'));sys.dont_write_bytecode=True
from import_meshy_characters import paint_blink_textures
from build_cheek_masks import smooth
ASSETS=ROOT/'assets/characters';OUT=ASSETS/'cheeks'
PROFILES=json.loads((OUT/'eyes.json').read_text())

def line_distance(x,z,a,b):
    delta=np.array(b)-a
    t=np.clip(((x-a[0])*delta[0]+(z-a[1])*delta[1])/np.dot(delta,delta),0,1)
    return np.hypot(x-a[0]-t*delta[0],z-a[1]-t*delta[1])

class Illustrator:
    def __init__(self,profile):
        self.profile=profile
        self.variants=[f'{phase}-{side}' for phase in ['hold','release'] for side in (['L','R'] if profile[phase][0]!=profile[phase][1] else ['L'])]
    def draw(self,key,side,pos,face,base,skin,alpha,lid):
        phase,grab=key.split('-');index=0 if side=='L' else 1
        # Source right eye appears on viewer/model left. Mirror the expression,
        # not the underlying skin, iris colours, hairstyle or cheek markings.
        source=index if grab=='L' else 1-index
        mode=self.profile[phase][source]
        sign=-1 if side=='L' else 1
        ex,eh=face['eye_x'],face['eye_half_height']
        dx=pos[:,0]-(face['center_x']+sign*ex);dz=pos[:,2]-face['eye_y']
        x=sign*dx/(ex*.85);z=dz/eh
        ink=np.median(lid['color'],axis=0)
        # Carry the original heavy upper lash and its broad outer wing along
        # the new contour. A generic thin line loses the character's identity.
        lo,hi=lid['span'];along=np.clip((x+.72)/1.72,0,1)
        source_x=(hi-along*(hi-lo)) if side=='L' else (lo+along*(hi-lo))
        source_brush=np.convolve(np.pad(lid['source_thickness'],7,mode='edge'),np.ones(15)/15,mode='valid')
        brush=np.clip(np.interp(source_x,lid['x'],source_brush)/eh*.95,.15,.70)
        taper=smooth((x+.78)/.14)*(1-smooth((x-.82)/.22))
        def upper_lash(height):
            band=(1-smooth((np.abs(z-height)-brush*.5)/.035))*taper
            return band
        if mode in ['angry','sad','flat','tired']:
            # Keep the model's actual iris and highlights under a redrawn lid.
            height={'sad':.60-.42*x*x+.10*x,'angry':.25-.28*x,'flat':.32+0*x,'tired':-.08+.14*x*x}[mode]
            erase=alpha*np.maximum(smooth((z-height+.08)/.15),smooth((np.abs(x)-.87)/.12))
            stroke=upper_lash(height)*alpha
            color=base*(1-erase[:,None])+skin*erase[:,None]
            return color*(1-stroke[:,None])+ink*stroke[:,None],np.maximum(erase,stroke)
        color=skin.copy();stroke=np.zeros(len(pos))
        if mode in ['squeeze','squeeze_soft','pressed']:
            # Rounded, thick > < contours; source Touch eyes are not a blink.
            opening={'squeeze':1,'squeeze_soft':.62,'pressed':.22}[mode]
            height=-.04+(x+.66)/1.46*.52*opening
            stroke=upper_lash(height)
            d=line_distance(x,z,(-.66,-.04),(.80,-.51*opening))
            stroke=np.maximum(stroke,1-smooth((d-.060)/.035))
            # Small crease above the inner corner adds the source's strained lid.
            d=line_distance(x,z,(-.65,.50),(-.27,.68))
            stroke=np.maximum(stroke,(1-smooth((d-.023)/.025))*.65)
        elif mode=='closed':
            path=-.20+.17*x*x
            stroke=upper_lash(path)
            for u in [.45,.68]:
                d=line_distance(x,z,(u,-.20+.17*u*u),(u+.12,-.34))
                stroke=np.maximum(stroke,1-smooth((d-.035)/.025))
        elif mode in ['blank','cloud','wide','cry','sparkle']:
            # Aya/Hilde's blank oval, Naia's four-lobed tearful white eye.
            angle=np.arctan2(z/.60,x/.73)
            radius=np.hypot(x/.73,z/.60)
            boundary=1.0-(.095*np.cos(angle*4) if mode=='cloud' else 0)
            fill=1-smooth((radius-boundary+.04)/.075)
            edge=1-smooth((np.abs(radius-boundary)-.045)/.05)
            color=color*(1-fill[:,None])+np.array([.99,.985,.975])*fill[:,None]
            stroke=edge
            upper=.60*np.sqrt(np.clip(1-(x/.80)**2,0,1))
            stroke=np.maximum(stroke,upper_lash(upper)*smooth((z-.10)/.15))
            if mode in ['wide','cry','sparkle']:
                iris_radius=np.hypot(x/(.45 if mode=='wide' else .64),(z+.02)/(.55 if mode=='wide' else .52))
                iris=1-smooth((iris_radius-.88)/.10)
                if mode=='wide':
                    iris_color=np.tile([.53,.48,.52],(len(pos),1));iris_color[z<-.14]=[.96,.91,.73]
                elif mode=='cry':iris_color=np.tile([.25,.13,.17],(len(pos),1))
                else:
                    top,bottom=([.35,.45,.70],[.53,.88,.90]) if side=='L' else ([.67,.37,.71],[.98,.89,.58])
                    iris_color=np.tile(top,(len(pos),1));iris_color[z<-.14]=bottom
                color=color*(1-iris[:,None])+iris_color*iris[:,None]
                shine=1-smooth((np.hypot((x+.22)/.19,(z-.24)/.22)-.85)/.15)
                shine=np.maximum(shine,1-smooth((np.hypot((x-.30)/.14,(z+.25)/.15)-.85)/.15))
                if mode=='wide':shine*=.8
                color=color*(1-shine[:,None])+np.array([1,.99,.98])*shine[:,None]
        else:raise ValueError(mode)
        color=color*(1-stroke[:,None])+ink*stroke[:,None]
        return base*(1-alpha[:,None])+color*alpha[:,None],alpha
    def save(self,key,pixels):
        # Use the existing native bake size and emit a complete PNG atomically.
        h,w=pixels.shape[:2]
        if max(w,h)>2048:
            image=bpy.data.images.new('Cheek atlas downsample',width=w,height=h,alpha=True)
            image.pixels.foreach_set(pixels.ravel());image.scale(2048,2048)
            w=h=2048;pixels=np.array(image.pixels[:],dtype=np.float32).reshape(h,w,4)
            bpy.data.images.remove(image)
        data=np.round(np.clip(pixels,0,1)*255).astype(np.uint8)
        def chunk(tag,buf):return struct.pack('>I',len(buf))+tag+buf+struct.pack('>I',zlib.crc32(tag+buf)&0xffffffff)
        raw=b''.join(b'\0'+row.tobytes() for row in data[::-1])
        png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>2I5B',w,h,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw,6))+chunk(b'IEND',b'')
        path=OUT/f"{self.profile['slug']}-{key}.png";tmp=path.with_suffix('.tmp');tmp.write_bytes(png);tmp.replace(path)
        print('CHEEK_EYE_DRAW_OK',path.name,flush=True)

if __name__=='__main__':
    selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [p['slug'] for p in PROFILES]
    for p in PROFILES:
        slug=p['slug']
        if slug not in selected:continue
        bpy.ops.wm.open_mainfile(filepath=str(ASSETS/f'{slug}.blend'))
        face=json.loads((ASSETS/f'{slug}-rig.json').read_text())['face']
        paint_blink_textures(bpy.data.objects[f'{slug}_SkinnedMesh'],face,slug,build_nodes=False,natural=True,illustrator=Illustrator(p))
