"""Bake illustrated Attack1_1 expressions into Naia's existing UV layout.

The face stays on the original skinned surface; no floating facial planes.
"""
import bpy, json, sys, struct, zlib, runpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];ASSETS=ROOT/'assets/characters'
bpy.ops.wm.open_mainfile(filepath=str(ASSETS/'naia.blend'))
mesh=bpy.data.objects['naia_SkinnedMesh'];mat=mesh.data.materials[0]
# Replacing an authored face pass must not stack drivers or stale packed images.
nodes=mat.node_tree.nodes;links=mat.node_tree.links;bsdf=nodes.get('Principled BSDF')
while bsdf.inputs['Base Color'].links and bsdf.inputs['Base Color'].links[0].from_node.label.startswith('Attack1_1 '):
    mix=bsdf.inputs['Base Color'].links[0].from_node
    previous=mix.inputs[1].links[0].from_socket;texture=mix.inputs[2].links[0].from_node
    mix.inputs[0].driver_remove('default_value');nodes.remove(mix);nodes.remove(texture)
    links.new(previous,bsdf.inputs['Base Color'])
for image in list(bpy.data.images):
    if image.name.startswith('naia-attack-') and image.users==0:bpy.data.images.remove(image)
face=json.loads((ASSETS/'naia-rig.json').read_text())['face']
base=next(n.image for n in mat.node_tree.nodes if n.type=='TEX_IMAGE' and n.image and n.image.name==mat['meshy_base_texture'])
w,h=base.size;pixels=np.array(base.pixels[:],dtype=np.float32).reshape(h,w,4)
outputs={key:pixels.copy() for key in ['wink','smile','squeeze','recover']}
coverage={key:np.zeros((h,w),dtype=np.float32) for key in outputs}
coords=np.array([v.co[:] for v in mesh.data.vertices]);uvs=np.array([u.uv[:] for u in mesh.data.uv_layers.active.data])
cx=face['center_x'];ez=face['eye_y'];ex=face['eye_x'];eh=face['eye_half_height'];mz=face['mouth_y']
def smooth(x):x=np.clip(x,0,1);return x*x*(3-2*x)
# Match the complexion used by the shipped closed-lid atlas.
ink=np.array([.17,.095,.12]);skin=np.array([.97,.84,.80]);pink=np.array([.90,.26,.32])
mesh.data.calc_loop_triangles()
tris=list(mesh.data.loop_triangles)
tree=BVHTree.FromPolygons([v.co for v in mesh.data.vertices],[t.vertices[:] for t in tris],all_triangles=True)
skin_samples=[]
for z in [ez-.015,ez-.025,ez-.035]:
    hit,_,index,_=tree.ray_cast(Vector((cx,-3,z)),Vector((0,1,0)))
    if hit:
        tri=tris[index];p=coords[list(tri.vertices)];uv=uvs[list(tri.loops)]
        bary=np.linalg.lstsq(np.vstack([p.T,np.ones(3)]),np.append(np.array(hit),1),rcond=None)[0]
        at=bary@uv;skin_samples.append(pixels[min(h-1,int(at[1]*h)),min(w-1,int(at[0]*w)),:3])
if skin_samples:skin=np.median(skin_samples,axis=0)
for tri in mesh.data.loop_triangles:
    p=coords[list(tri.vertices)]
    if p[:,2].max()<mz-.07 or p[:,2].min()>ez+eh*1.7 or p[:,1].min()>-face['front']+.10:continue
    uv=uvs[list(tri.loops)]*np.array([w,h]);low=np.maximum(0,np.floor(uv.min(0)).astype(int));high=np.minimum([w-1,h-1],np.ceil(uv.max(0)).astype(int))
    if np.prod(high-low+1)>100000:continue
    matrix=np.vstack([uv.T,np.ones(3)])
    if abs(np.linalg.det(matrix))<1e-8:continue
    xx,yy=np.meshgrid(np.arange(low[0],high[0]+1),np.arange(low[1],high[1]+1));ix=xx.ravel();iy=yy.ravel()
    bary=np.linalg.solve(matrix,np.stack([ix+.5,iy+.5,np.ones(xx.size)])).T
    pos=bary@p;inside=np.all(bary>=-.001,axis=1)&(pos[:,1]<-face['front']+.10)
    for key in outputs:
        for sign in [-1,1]:
            # Source Eye_R is the wink (the viewer's left in the reference).
            if key in ['wink','smile'] and sign>0:continue
            dx=pos[:,0]-(cx+sign*ex);dz=pos[:,2]-ez;rx=ex*.83;rz=eh*1.25
            alpha=(1-smooth((np.abs(dx)/rx-.83)/.17))*(1-smooth((np.abs(dz)/rz-.78)/.22))*inside
            active=alpha>0
            # The generated skin patch clears iris + the previous closed-lid stroke.
            local_skin=skin+np.clip(dz[:,None]/rz,-1,1)*.018
            if key in ['wink','smile']:
                line=.014-.020*(dx/(rx*.75))**2
            else:
                along=sign*dx+.040
                line=.50*along
            if key in ['squeeze','recover']:
                # Two diagonal strokes meet inward, reproducing > < scissors eyes.
                line_dist=np.abs(np.abs(dz-.004)-line)
            else:line_dist=np.abs(dz-line)
            stroke=(1-smooth((line_dist-.0028)/.003))*(1-smooth((np.abs(dx)/rx-.76)/.10))
            if key in ['squeeze','recover']:stroke*=smooth((along+.002)/.004)
            color=local_skin*(1-stroke[:,None])+ink*stroke[:,None]
            outputs[key][iy[active],ix[active],:3]=pixels[iy[active],ix[active],:3]*(1-alpha[active,None])+color[active]*alpha[active,None]
            coverage[key][iy[active],ix[active]]=np.maximum(coverage[key][iy[active],ix[active]],alpha[active])
        if key not in ['smile','squeeze']:continue
        dx=pos[:,0]-cx;dz=pos[:,2]-mz
        alpha=(1-smooth((np.abs(dx)/.077-.70)/.30))*(1-smooth((np.abs(dz)/.058-.70)/.30))*inside
        active=alpha>0
        # Rounded open happy mouth, with a shallow notched upper lip.
        top=.010+.007*np.abs(dx/.045);bottom=-.035*np.sqrt(np.clip(1-(dx/.044)**2,0,1))
        mouth=(1-smooth((np.abs(dx)-.043)/.004))*(1-smooth((dz-top)/.004))*smooth((dz-bottom)/.004)
        tongue=smooth((-.010-dz)/.010)
        mouth_color=ink*(1-tongue[:,None])+pink*tongue[:,None]
        color=skin*(1-mouth[:,None])+mouth_color*mouth[:,None]
        outputs[key][iy[active],ix[active],:3]=pixels[iy[active],ix[active],:3]*(1-alpha[active,None])+color[active]*alpha[active,None]
        coverage[key][iy[active],ix[active]]=np.maximum(coverage[key][iy[active],ix[active]],alpha[active])
for key,pixels_out in outputs.items():
    mask=coverage[key]>.85
    for _ in range(3):
        expanded=mask.copy()
        for src,dst in [((slice(1,None),slice(None)),(slice(None,-1),slice(None))),((slice(None,-1),slice(None)),(slice(1,None),slice(None))),((slice(None),slice(1,None)),(slice(None),slice(None,-1))),((slice(None),slice(None,-1)),(slice(None),slice(1,None)))]:
            fill=mask[src]&~expanded[dst];pixels_out[dst][fill,:3]=pixels_out[src][fill,:3];expanded[dst]|=fill
        mask=expanded
    image=bpy.data.images.new('naia-attack-'+key,width=w,height=h,alpha=True)
    image.pixels.foreach_set(pixels_out.ravel());image.scale(2048,2048)
    # Encode explicitly and atomically. Blender's generated-image save produced a
    # truncated first PNG on this host; a complete IEND/CRC is required by Fyrox.
    rgba=np.array(image.pixels[:],dtype=np.float32).reshape(2048,2048,4)
    # These generated atlas buffers already contain the source texture values.
    # Applying a second display transfer would wash out every unchanged texel.
    rgba=np.round(np.clip(rgba,0,1)*255).astype(np.uint8)
    def chunk(tag,data):return struct.pack('>I',len(data))+tag+data+struct.pack('>I',zlib.crc32(tag+data)&0xffffffff)
    raw=b''.join(b'\x00'+row.tobytes() for row in rgba[::-1])
    png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>2I5B',2048,2048,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw,6))+chunk(b'IEND',b'')
    path=ASSETS/f'naia-attack-{key}.png';temporary=path.with_suffix('.tmp.png')
    temporary.write_bytes(png);temporary.replace(path)
    image.filepath_raw=str(path);image.source='FILE';image.reload()
    print('NAIA_ATTACK_FACE_TEXTURE',key,flush=True)
    # Author the same attachment switches into the editable Blender material.
    nodes=mat.node_tree.nodes;links=mat.node_tree.links;bsdf=nodes.get('Principled BSDF')
    previous=bsdf.inputs['Base Color'].links[0].from_socket
    texture=nodes.new('ShaderNodeTexImage');texture.image=image;image.pack()
    mix=nodes.new('ShaderNodeMixRGB');mix.label='Attack1_1 '+key
    links.new(previous,mix.inputs[1]);links.new(texture.outputs['Color'],mix.inputs[2]);links.new(mix.outputs[0],bsdf.inputs['Base Color'])
    start,end={'wink':(.26,.48),'smile':(.48,.5),'squeeze':(.5,.76),'recover':(.76,.78)}[key]
    mix.inputs[0].driver_add('default_value').driver.expression=f'1.0 if {156+24*start} <= frame < {156+24*end} else 0.0'

detail=json.loads((ASSETS/'attack/naia-detail.json').read_text())
keys=mesh.data.shape_keys.key_blocks
names=['eyeBlink_L','eyeBlink_R','mouthSmile_L','mouthSmile_R','jawOpen']
for frame in [0,155,181,528]:
    for name in names:keys[name].value=0;keys[name].keyframe_insert('value',frame=frame)
for i,sample in enumerate(detail['samples']):
    values={name:0 for name in names}
    for side,source in [('L',1),('R',0)]:
        part=sample['eyes'][source]
        values['eyeBlink_'+side]=1 if any(n in part['attachment'] for n in ['Scissors','Happy']) else max(0,min(1,1-part['scale']))
    if sample['mouth']['attachment']=='F_Mouth_Happy2':
        values.update(mouthSmile_L=.6,mouthSmile_R=.6,jawOpen=min(.4,.22*sample['mouth']['scale']))
    for name,value in values.items():keys[name].value=value;keys[name].keyframe_insert('value',frame=156+24*i/100)
scene=bpy.context.scene;scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'naia.blend'))
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
export_fbx(mesh,bpy.data.objects['TSB_Rig'],ASSETS/'naia.fbx')
# The FBX exporter may rebuild vertex order; always refresh its sparse morph map.
saved_argv=sys.argv
sys.argv=[str(ROOT/'scripts/export_meshy_morphs.py'),'--',str(ASSETS),'naia']
runpy.run_path(str(ROOT/'scripts/export_meshy_morphs.py'),run_name='__main__')
sys.argv=saved_argv
