"""Sample each character's actual upper lash band, then lower its curved silhouette."""
import numpy as np
from mathutils import Vector
from mathutils.geometry import barycentric_transform

def sample_lids(tree,tris,coords,uvs,pixels,face,skip=()):
    height,width=pixels.shape[:2];result={}
    ex,eh,ez,cx=[face[k] for k in ['eye_x','eye_half_height','eye_y','center_x']]
    for side,sign in [('L',-1),('R',1)]:
        if side in skip:continue
        xs=np.linspace(cx+sign*ex-ex*1.22,cx+sign*ex+ex*1.22,112)
        zs=np.linspace(ez+eh*.30,ez+eh*1.48,72)
        levels=[];thickness=[];colors=[];depths=[];valid=[]
        for x in xs:
            dark=[];rgb=[];ys=[]
            lower,_,_,_=tree.ray_cast(Vector((x,-3,ez-eh*.3)),Vector((0,1,0)))
            for z in zs:
                hit,_,index,_=tree.ray_cast(Vector((x,-3,z)),Vector((0,1,0)))
                if hit is None:dark.append(False);rgb.append([.1,.07,.09]);ys.append(0);continue
                tri=tris[index];p=coords[list(tri.vertices)];uv=uvs[list(tri.loops)]
                at=barycentric_transform(hit,*[Vector(v) for v in p],*[Vector((u,v,0)) for u,v in uv])
                color=pixels[min(height-1,max(0,int(at.y*height))),min(width-1,max(0,int(at.x*width))),:3]
                dark.append(bool(max(color)<.38 and (lower is None or hit.y>lower.y-.055)))
                rgb.append(color);ys.append(hit.y)
            runs=[];start=None
            for i,on in enumerate([*dark,False]):
                if on and start is None:start=i
                if not on and start is not None:runs.append((start,i));start=None
            a,b=max(runs,key=lambda run:run[1]-run[0]) if runs else (0,0)
            valid.append(b-a>=2)
            levels.append(float(np.mean(zs[a:b])) if b>a else ez+eh*.8)
            thickness.append(float((b-a)*(zs[1]-zs[0])))
            colors.append(np.mean(rgb[a:b],axis=0) if b>a else [.1,.07,.09])
            depths.append(float(np.mean(ys[a:b])) if b>a else -face['front'])
        good=np.flatnonzero(valid)
        if len(good)<12:raise RuntimeError(f'Cannot sample {side} eyelash: only {len(good)} columns')
        indices=np.arange(len(xs))
        level=np.interp(indices,good,np.array(levels)[good]);thick=np.interp(indices,good,np.array(thickness)[good])
        # Keep the source's thickness and swept outer ends, with a relaxed lower arc.
        level=np.convolve(np.pad(level,2,mode='edge'),[.1,.2,.4,.2,.1],mode='valid')
        thick=np.convolve(np.pad(thick,2,mode='edge'),[.1,.2,.4,.2,.1],mode='valid')
        arc=((xs-(xs[good[0]]+xs[good[-1]])*.5)/max((xs[good[-1]]-xs[good[0]])*.5,.01))**2
        closed=ez-eh*.24+eh*.24*arc-.18*(level-np.median(level[good]))
        result[side]=dict(x=xs,open=level,closed=closed,source_thickness=thick,thickness=np.clip(thick*.62+.0015,.0045,.016),
            color=np.array([np.interp(indices,good,np.array(colors)[good,c]) for c in range(3)]).T,
            depth=np.interp(indices,good,np.array(depths)[good]),span=(xs[good[0]],xs[good[-1]]))
    return result

def lid_samples(profile,x,closure):
    height=np.interp(x,profile['x'],profile['open']*(1-closure)+profile['closed']*closure)
    thickness=np.interp(x,profile['x'],profile['thickness'])
    color=np.stack([np.interp(x,profile['x'],profile['color'][:,c]) for c in range(3)],axis=1)
    lo,hi=profile['span'];span=np.clip((x-lo)/.008,0,1)*np.clip((hi-x)/.008,0,1)
    return height,thickness,color,span
