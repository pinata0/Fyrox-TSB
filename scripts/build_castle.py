"""Blender 5.x: build editable castle scenery, optimized FBX and reproducible previews.

blender --background --python scripts/build_castle.py [-- --render]
Collision boxes are read from the exact layout embedded in Rust. Reference images
are research only, never textures. Blender coordinates (x,-game_z,game_y).
"""
import bpy
import bmesh
import json
import math
import random
import sys
import struct
import zlib
from pathlib import Path
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from castle_gate_arch import lintel_geometry, moulding_points
OUT=ROOT/'assets/maps/fairy-kingdom-castle'
ART=ROOT/'artifacts/fairy-kingdom-castle'
OUT.mkdir(parents=True,exist_ok=True); ART.mkdir(parents=True,exist_ok=True)
# A uniform emission mask; no supplied illustration is baked onto geometry.
def png_chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
(OUT/'window-emission.png').write_bytes(b'\x89PNG\r\n\x1a\n'+png_chunk(b'IHDR',struct.pack('>2I5B',1,1,8,2,0,0,0))+png_chunk(b'IDAT',zlib.compress(b'\x00\xff\xff\xff'))+png_chunk(b'IEND',b''))
layout=json.loads((OUT/'layout.json').read_text(encoding='utf-8'))
random.seed(37)
if hasattr(bpy.context.preferences.filepaths, 'file_preview_type'):
    bpy.context.preferences.filepaths.file_preview_type='NONE'
bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
for c in list(bpy.data.collections):
    if c.name != 'Collection': bpy.data.collections.remove(c)
COL=None
def collection(name):
    global COL
    COL=bpy.data.collections.new(name); bpy.context.scene.collection.children.link(COL)
    print('CASTLE_SECTION',name,flush=True)
    return COL
def adopt(obj,name,mat):
    obj.name=name
    for c in list(obj.users_collection): c.objects.unlink(obj)
    COL.objects.link(obj)
    if mat: obj.data.materials.append(mat)
    return obj
def linear(x): return x/12.92 if x <= .04045 else ((x+.055)/1.055)**2.4
def material(name,color,metal=0):
    m=bpy.data.materials.new(name); m.use_nodes=True
    m['srgb']=color
    rgba=tuple(linear(v/255) for v in color)+(1,)
    m.diffuse_color=rgba
    bs=m.node_tree.nodes.get('Principled BSDF'); bs.inputs['Base Color'].default_value=rgba
    bs.inputs['Roughness'].default_value=.7; bs.inputs['Metallic'].default_value=metal
    return m
IVORY=material('Ivory limestone',[231,235,220]); STONE=material('Sage stone',[158,189,184])
TEAL=material('Patinated mint roofs',[76,167,158]); DARK=material('Deep teal',[39,103,106])
GOLD=material('Warm gilding',[227,184,88],.25); ROSE=material('Royal rose',[200,114,144])
LEAF=material('World tree jade',[92,166,130]); LEAF2=material('World tree mint',[139,200,150])
BARK=material('World tree bark',[117,131,132]); PAVE=material('Warm courtyard paving',[222,217,197])
PALE=material('Light courtyard paving',[240,234,211]); GRASS=material('Garden grass',[131,169,128])
LIGHT=material('Lantern glass',[255,227,157]); RUBY=material('Crown jewel',[115,187,212])
MATS={tuple(m['srgb']):m for m in [IVORY,STONE,TEAL,GOLD]}
def loc(p): return (p[0],-p[2],p[1])
def cube(name,p,size,mat,bevel=0):
    mesh=bpy.data.meshes.new(name)
    verts=[(x*size[0]/2,y*size[2]/2,z*size[1]/2) for x,y,z in [(-1,-1,-1),(-1,-1,1),(-1,1,-1),(-1,1,1),(1,-1,-1),(1,-1,1),(1,1,-1),(1,1,1)]]
    mesh.from_pydata(verts,[],[(0,4,6,2),(1,3,7,5),(0,1,5,4),(2,6,7,3),(0,2,3,1),(4,5,7,6)])
    o=bpy.data.objects.new(name,mesh);o.location=loc(p);COL.objects.link(o);mesh.materials.append(mat)
    if bevel:
        bm=bmesh.new();bm.from_mesh(mesh)
        bmesh.ops.bevel(bm,geom=list(bm.edges),offset=min(bevel,min(size)*.2),segments=2,affect='EDGES')
        bm.to_mesh(mesh);bm.free()
    return o
def sphere(name,p,size,mat,sub=2):
    mesh=bpy.data.meshes.new(name);bm=bmesh.new();bmesh.ops.create_icosphere(bm,subdivisions=sub,radius=1)
    bm.to_mesh(mesh);bm.free()
    o=bpy.data.objects.new(name,mesh);o.location=loc(p);COL.objects.link(o);mesh.materials.append(mat)
    o.scale=(size[0],size[2],size[1])
    return o
def cylinder(name,p,r,h,mat,vertices=16,r2=None):
    o=lathe(name,(0,-h/2,0),[(0,r),(h,r if r2 is None else r2)],mat,n=vertices)
    o.location=loc(p)
    return o
def beam(name,a,b,r,mat):
    av,bv=Vector(loc(a)),Vector(loc(b)); d=bv-av
    o=cylinder(name,(0,0,0),r,d.length,mat,10,r*.75)
    o.location=(av+bv)/2;o.rotation_euler=d.to_track_quat('Z','Y').to_euler(); return o
def lathe(name,p,profile,mat,rx=1,rz=1,n=24):
    verts=[]; faces=[]
    for h,r in profile:
        for j in range(n):
            t=j*2*math.pi/n; verts.append((p[0]+r*rx*math.cos(t),-p[2]+r*rz*math.sin(t),p[1]+h))
    for i in range(len(profile)-1):
        for j in range(n):
            a=i*n+j; b=i*n+(j+1)%n; faces.append((a,b,b+n,a+n))
    faces.extend([tuple(reversed(range(n))),tuple((len(profile)-1)*n+j for j in range(n))])
    mesh=bpy.data.meshes.new(name); mesh.from_pydata(verts,[],faces); mesh.update()
    o=bpy.data.objects.new(name,mesh); COL.objects.link(o); mesh.materials.append(mat)
    for f in mesh.polygons: f.use_smooth=True
    return o
def dome(name,x,y,z,r,h,rz=1):
    lathe(name+' eave',(x,y,z),[(0,r*1.07),(.18,r*1.07),(.25,r)],GOLD,rz=rz)
    profile=[(.2,r),(.25+h*.24,r*.96),(.25+h*.5,r*.79),(.25+h*.75,r*.46),(.25+h,.06)]
    lathe(name+' mint dome',(x,y,z),profile,TEAL,rz=rz)
    cylinder(name+' finial',(x,y+h+.5,z),.16,.65,GOLD,r2=0)
    for j in range(8):
        angle=j*math.pi/4
        for (ya,ra),(yb,rb) in zip(profile,profile[1:]):
            beam(name+' roof rib',(x+ra*math.cos(angle),y+ya,z+ra*rz*math.sin(angle)),
                 (x+rb*math.cos(angle),y+yb,z+rb*rz*math.sin(angle)),.035,GOLD)
def crown(name,x,y,z,scale=1):
    cube(name+' band',(x,y,z),(2.4*scale,.36*scale,.3*scale),GOLD,.05)
    for dx,h in [(-1,.7),(0,1.15),(1,.7)]:
        cylinder(name+' point',(x+dx*scale,y+h*scale/2,z),.38*scale,h*scale,GOLD,4,0)
        sphere(name+' pearl',(x+dx*scale,y+h*scale,z),(.13*scale,)*3,PALE,1)
    sphere(name+' jewel',(x,y+.2*scale,z-.18*scale),(.25*scale,.34*scale,.1*scale),RUBY,1)
def banner(name,x,y,z,scale=1):
    cube(name+' cloth',(x,y,z),(.75*scale,1.75*scale,.055),TEAL)
    cube(name+' hem',(x,y-.8*scale,z-.035),(.76*scale,.12,.05),GOLD)
    beam(name+' rod',(x-.55*scale,y+.95*scale,z),(x+.55*scale,y+.95*scale,z),.045,GOLD)
    crown(name+' emblem',x,y,z-.06,.21*scale)
def tower(name,x,z,height,r=2):
    cylinder(name+' base',(x,.25,z),r*1.1,.5,STONE)
    cylinder(name+' shaft',(x,height/2,z),r,height,IVORY)
    for y in [.7,height*.48,height-.4]: cylinder(name+' ring',(x,y,z),r*1.05,.25,STONE)
    dome(name,x,height,z,r*1.1,r*1.6)
    banner(name,x,height*.65,z-r-.03,1.15)
def lamp(name,x,z,y=0):
    cylinder(name+' foot',(x,y+.15,z),.28,.3,STONE)
    cylinder(name+' stem',(x,y+1.45,z),.08,2.6,GOLD,8)
    cylinder(name+' glass',(x,y+2.8,z),.25,.5,LIGHT,6)
    cylinder(name+' cap',(x,y+3.15,z),.38,.3,TEAL,6,0)
def tree(name,x,z,y=0,scale=1):
    cylinder(name+' trunk',(x,y+1.2*scale,z),.22*scale,2.4*scale,BARK,10)
    for dx,dy,dz,s in [(0,2.8,0,1),(-.5,2.5,.3,.8),(.45,2.7,-.3,.85)]:
        sphere(name+' leaves',(x+dx*scale,y+dy*scale,z+dz*scale),(s*scale,.9*s*scale,s*scale),LEAF2)

# The skin consumes the same authored topology as the simulation.
exec(compile((ROOT/'scripts/castle_scenery.py').read_text(encoding='utf-8'), 'castle_scenery.py', 'exec'), globals())

collection('09 Lighting and cameras')
scene=bpy.context.scene; scene.render.engine='CYCLES';scene.cycles.samples=24
scene.world.color=(.35,.42,.45)
scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.55,.7,.8,1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value=.65
bpy.ops.object.light_add(type='SUN',location=(0,0,25));sun=bpy.context.object;sun.name='Warm morning sun'
sun.rotation_euler=(math.radians(28),math.radians(-25),math.radians(-25));sun.data.energy=3;sun.data.angle=.12
for i,light in enumerate(layout.get('lights',[])):
    bpy.ops.object.light_add(type='POINT',location=loc(light['position']))
    o=bpy.context.object;o.name='Concept room fill %02d'%i
    o.data.color=tuple(v/255 for v in light['color']);o.data.energy=light['intensity']*450
    o.data.shadow_soft_size=2.0
scene.view_settings.view_transform='Standard'
scene.render.resolution_x=1440;scene.render.resolution_y=1000;scene.render.resolution_percentage=100
def camera(name,p,target,lens=32,ortho=None):
    bpy.ops.object.camera_add(location=loc(p));o=bpy.context.object;o.name=name
    o.rotation_euler=(Vector(loc(target))-o.location).to_track_quat('-Z','Y').to_euler()
    o.data.lens=lens;o.data.clip_end=500
    if ortho: o.data.type='ORTHO';o.data.ortho_scale=ortho
    return o
cameras=[camera('Overview',(-65,75,-90),(20,3,-2),42),camera('A gate',(2,2,-17),(5,5,12),24),
         camera('B royal court',(29.5,2,15),(66,7,15),22),camera('Layout',(23,120,-8),(23,0,-8),32,150)]
concept_cameras=[
    ('bedroom',(5,1.8,-41.6),(4,1.7,-46),18),
    ('study',(-11.8,1.8,-18),(-12,1.5,-12),22),
    ('street',(1,1.8,-38),(2,3,-18),24),
    ('cafe',(1,1.8,-24),(13,2.5,-20),24),
    ('square',(5,1.8,-40),(16,3.5,-35),24),
    ('atrium',(11.6,1.8,10.8),(14,3.4,14),16),
    ('corridor',(26.5,1.8,-1.5),(43,3,-1),24),
    ('guardian-hall',(26,1.8,30),(37,3,28),20),
    ('throne',(43,1.8,15),(53,3.2,15),24),
]
for name,p,target,lens in concept_cameras:camera('Concept '+name,p,target,lens)
scene.camera=cameras[0]
zero_faces=0
for o in list(scene.objects):
    if o.type!='MESH': continue
    mesh=o.data;bm=bmesh.new();bm.from_mesh(mesh)
    bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=0.000001)
    bmesh.ops.dissolve_degenerate(bm,edges=list(bm.edges),dist=0.000001)
    bmesh.ops.triangulate(bm,faces=list(bm.faces))
    bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
    zero_faces+=sum(f.calc_area()<1e-10 for f in bm.faces)
    bm.to_mesh(mesh);bm.free();mesh.update()
    uv=mesh.uv_layers.new(name='Planar detail UV')
    for face in mesh.polygons:
        axis=max(range(3),key=lambda i:abs(face.normal[i]));axes=[i for i in range(3) if i!=axis]
        for li in face.loop_indices:
            co=mesh.vertices[mesh.loops[li].vertex_index].co
            uv.data[li].uv=(co[axes[0]]*.25,co[axes[1]]*.25)
assert zero_faces==0, f'Degenerate model faces: {zero_faces}'
print('CASTLE_MESH_QA_OK: no zero-area triangles; planar UVs and normals generated',flush=True)
scene['map_slug']=layout['slug'];scene['layout_source']='layout.json';scene['reference_manifest']='../../references/fairy-kingdom-castle/sources.json'
scene['coordinate_system']='Blender (x,-game_z,game_y), FBX metres imported at 0.01 in Fyrox'
scene['concept_manifest']='../../references/fairy-kingdom-castle/concept/manifest.json'
scene['layout_revision']=layout['revision']
for area in bpy.context.screen.areas:
    if area.type=='VIEW_3D':
        area.spaces.active.region_3d.view_distance=125
        area.spaces.active.region_3d.view_location=(23,8,5)
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'fairy-kingdom-castle.blend'))
mesh_objects=[o for o in scene.objects if o.type=='MESH']
stats={'objects':len(mesh_objects),'vertices':sum(len(o.data.vertices) for o in mesh_objects),
       'triangles':sum(sum(len(p.vertices)-2 for p in o.data.polygons) for o in mesh_objects),
       'collision_blocks':len(layout['blocks']),'materials':len(bpy.data.materials),'slug':layout['slug']}

# Export per spatial region/material to reduce draw calls without losing culling.
bpy.ops.object.select_all(action='DESELECT')
groups={}
for o in mesh_objects:
    center=sum((o.matrix_world@Vector(c) for c in o.bound_box),Vector())/8
    key=(math.floor(center.x/12),math.floor(-center.y/16),o.data.materials[0].name if o.data.materials else '')
    groups.setdefault(key,[]).append(o)
exports=[]
for key,objects in groups.items():
    bpy.ops.object.select_all(action='DESELECT')
    for o in objects: o.select_set(True)
    bpy.context.view_layer.objects.active=objects[0]
    if len(objects)>1: bpy.ops.object.join()
    o=bpy.context.object;o.name='Castle chunk %s %s %s'%key;exports.append(o)
for m in bpy.data.materials:
    if 'srgb' in m:
        rgba=tuple(v/255 for v in m['srgb'])+(1,)
        m.diffuse_color=rgba;m.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=rgba
bpy.ops.object.select_all(action='DESELECT')
for o in exports:o.select_set(True)
bpy.ops.export_scene.fbx(filepath=str(OUT/'fairy-kingdom-castle.fbx'),use_selection=True,object_types={'MESH'},
                         apply_unit_scale=True,axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
stats['export_chunks']=len(exports)
(OUT/'build-report.json').write_text(json.dumps(stats,indent=2)+'\n',encoding='utf-8')
print('CASTLE_MODEL_OK',json.dumps(stats),flush=True)
if '--render' in sys.argv or '--concept-render' in sys.argv:
    # Restore the fully editable source and correct linear colors for Blender previews.
    bpy.ops.wm.open_mainfile(filepath=str(OUT/'fairy-kingdom-castle.blend'))
    scene=bpy.context.scene
    shots=[('Concept '+name,'concept-'+name) for name,*_ in concept_cameras]
    if '--render' in sys.argv:shots += [('Overview','blender-overview'),('A gate','blender-gate-a'),('B royal court','blender-court-b'),('Layout','blender-layout')]
    for name,filename in shots:
        scene.camera=bpy.data.objects[name];scene.render.filepath=str(ART/(filename+'.png'))
        bpy.ops.render.render(write_still=True)
    print('CASTLE_RENDER_OK',flush=True)
