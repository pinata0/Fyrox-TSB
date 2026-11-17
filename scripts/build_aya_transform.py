"""Preserve Aya's textured parasol and add editable folding morphs and three solid blades.

Run in Blender. The original mesh/UVs are retained; the shaft is aligned to Grip.
Morphs provide continuous folding of the existing canopy without cutting its UV seams.
"""
import bpy, bmesh, json, math, sys
import numpy as np
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/aya-transform'
ASSETS=ROOT/'assets/characters'
bpy.ops.wm.open_mainfile(filepath=str(OUT/'before/assets/characters/aya-weapon.blend'))
mesh=bpy.data.objects['aya_weapon']
p=np.array([v.co[:] for v in mesh.data.vertices])
# Fit the narrow shaft below the canopy, keeping the actual grip at the origin.
low=p[p[:,2]<.20]
fit=np.array([np.polyfit(low[:,2],low[:,i],1) for i in [0,1]])
axis=Vector((fit[0,0],fit[1,0],1)).normalized()
grip=Vector((np.polyval(fit[0],-.13),np.polyval(fit[1],-.13),-.13))
rot=axis.rotation_difference(Vector((0,0,1))).to_matrix()
p=np.array([rot@(Vector(v)-grip) for v in p])
mesh.data.vertices.foreach_set('co',p.astype(np.float32).ravel());mesh.data.update()
rad=np.linalg.norm(p[:,:2],axis=1)
hub=float(np.percentile(p[rad<.06,2],95))-.035
# Smooth weights keep the shaft rigid and preserve the hub connection.
w=np.clip((rad-.045)/.08,0,1);w=w*w*(3-2*w)
fold=p.copy()
fold[:,:2]*=(1-w*.88)[:,None]
fold[:,2]=p[:,2]*(1-w)+(hub-.045-np.sqrt(rad**2+(p[:,2]-hub)**2)*.40)*w
wide=fold.copy();wide[:,:2]*=(1+w*.65)[:,None]
mesh.shape_key_add(name='Basis')
for name,points in [('parasolFold',fold),('parasolSpread',p+(wide-fold))]:
 k=mesh.shape_key_add(name=name);k.data.foreach_set('co',points.astype(np.float32).ravel());k.value=0
for im in bpy.data.images:
 if im.source=='FILE':
  im.filepath=str(ASSETS/'aya-weapon-meshy-basecolor.png');im.pack()
def mat(name,color,metal):
 m=bpy.data.materials.new(name);m.diffuse_color=(*color,1);m.use_nodes=True
 bs=m.node_tree.nodes.get('Principled BSDF');bs.inputs['Base Color'].default_value=(*color,1);bs.inputs['Metallic'].default_value=metal;bs.inputs['Roughness'].default_value=.28
 return m
pearl=mat('Aya pearl blade',(.82,.91,.97),.35)
edge=mat('Aya silver cutting edge',(.96,.98,1),.65)
ridge=mat('Aya blue steel ridge',(.45,.65,.78),.45)
# Author a beveled, double-sided blade from an outline. These are original 3D
# interpretations of the archived Blade_1/2/3 silhouettes, not sprite billboards.
profiles=[
 [(-.040,0),(.047,0),(.052,.20),(.082,.48),(.147,.78),(.182,.94),(.102,.88),(.027,.69),(-.035,.42),(-.044,.16)],
 [(-.075,0),(.065,0),(.090,.42),(.080,.84),(.045,1.10),(-.018,1.16),(-.065,.95),(-.072,.40)],
 [(-.050,0),(.070,0),(.130,.14),(.140,.30),(.092,.38),(.125,.47),(.232,.44),(.268,.61),(.200,.81),(.290,1.14),(.105,1.02),(-.065,.82),(-.165,.55),(-.180,.30),(-.125,.08)]
]
blade_objects=[]
for number,outline in enumerate(profiles,1):
 # Round silhouette corners while retaining the highest cutting tip.
 rounded=[]
 for i,point in enumerate(outline):
  p0=np.array(outline[i-1]);p1=np.array(point);p2=np.array(outline[(i+1)%len(outline)])
  if point[1]==max(q[1] for q in outline):rounded.append(point);continue
  a=p1*.78+p0*.22;b=p1*.78+p2*.22
  for t in [0,.25,.5,.75,1]:rounded.append(tuple((1-t)**2*a+2*(1-t)*t*p1+t*t*b))
 outline=rounded
 n=len(outline);center=np.mean(outline,axis=0);vertices=[]
 for depth,shrink in [(-.026,.82),(0,1),(.026,.82)]:
  for x,z in outline:
   a=center+(np.array([x,z])-center)*shrink
   vertices.append((float(a[0]),depth,float(a[1])+hub-.09))
 faces=[tuple(reversed(range(n))),tuple(range(2*n,3*n))]
 for ring in range(2):
  for i in range(n):faces.append((ring*n+i,ring*n+(i+1)%n,(ring+1)*n+(i+1)%n,(ring+1)*n+i))
 data=bpy.data.meshes.new(f'Aya Blade {number}');data.from_pydata(vertices,[],faces);data.update()
 o=bpy.data.objects.new(f'Aya_Blade_{number}',data);bpy.context.collection.objects.link(o)
 for m in [pearl,edge,ridge]:data.materials.append(m)
 for f in data.polygons:f.material_index=0 if f.index<2 else 1
 # Explicit outward normals and triangulation are required for the concave
 # hooked blade: the FBX importer cannot safely fan-triangulate its notch.
 bm=bmesh.new();bm.from_mesh(data);bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(data);bm.free()
 o.modifiers.new('Triangulated export for concave blade','TRIANGULATE')
 # Store geometry relative to the hilt, so emergence scales out from the canopy.
 for v in data.vertices:v.co.z-=hub-.09
 o.location.z=hub-.09
 blade_objects.append(o)
for name in ['PropRoot','Grip','Muzzle']:
 o=bpy.data.objects.get(name)
 if o:o.location=(0,0,0 if name!='Muzzle' else hub+1)
scene=bpy.context.scene;scene.render.fps=30;scene.frame_start=0;scene.frame_end=178
records=json.loads((ASSETS/'attack/aya-transform.json').read_text())
# Authoring preview: normal 0..45, enhanced 90..178. Runtime uses the same samples.
for record,start in zip(records,[0,90]):
 for i,s in enumerate(record['samples']):
  frame=start+i
  for name,value in [('parasolFold',s['fold']),('parasolSpread',s['spread'])]:
   k=mesh.data.shape_keys.key_blocks[name];k.value=value;k.keyframe_insert(data_path='value',frame=frame)
  for number,o in enumerate(blade_objects,1):
   amount=s['blade_alpha'] if number==s['blade'] else 0
   o.scale=(amount,amount,amount);o.keyframe_insert(data_path='scale',frame=frame)
for action in bpy.data.actions:
 for layer in action.layers:
  for strip in layer.strips:
   for bag in strip.channelbags:
    for c in bag.fcurves:
     for k in c.keyframe_points:k.interpolation='LINEAR'
scene.frame_set(0)
scene['aya_transform_source']='Attack1_1 / Attack2_1; 12-petal source fold, Blade_1/2/3 attachment timing'
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'aya-weapon.blend'),relative_remap=False)
# Export all blades at authored size. Visibility and emergence are driven per instance.
mesh.data.shape_keys.animation_data_clear()
for k in mesh.data.shape_keys.key_blocks:k.value=0
for o in blade_objects:o.animation_data_clear();o.scale=(1,1,1)
bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True)
for o in blade_objects:o.select_set(True)
bpy.context.view_layer.objects.active=mesh
bpy.ops.export_scene.fbx(filepath=str(ASSETS/'aya-weapon.fbx'),use_selection=True,object_types={'MESH'},axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
(OUT/'weapon-build.json').write_text(json.dumps({'original_vertices':len(p),'topology_and_uv_preserved':True,'hub':hub,'grip':list(grip),'fold_morphs':['parasolFold','parasolSpread'],'blades':3},indent=2))
prop_path=ASSETS/'aya-weapon-prop.json'
prop=json.loads(prop_path.read_text());prop.update(vertices=sum(len(o.data.vertices) for o in [mesh,*blade_objects]),triangles=sum(sum(len(f.vertices)-2 for f in o.data.polygons) for o in [mesh,*blade_objects]),fold_morphs=['parasolFold','parasolSpread'],blade_meshes=[o.name for o in blade_objects],bounds=[p.min(0).tolist(),p.max(0).tolist()],bounds_state='open parasol, blades hidden')
prop_path.write_text(json.dumps(prop,indent=2))
print('AYA_TRANSFORM_MODEL_OK',len(p),hub,flush=True)
