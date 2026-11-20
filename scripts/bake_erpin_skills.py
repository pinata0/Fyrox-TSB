"""Append archived Erpin E/Q motion, preserving existing meshes, face and old keys."""
import bpy,bmesh,json,math,sys,shutil,hashlib
import numpy as np
from pathlib import Path
from mathutils import Vector,Matrix
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';WORK=ROOT/'artifacts/erpin-skills';BEFORE=WORK/'before';BEFORE.mkdir(parents=True,exist_ok=True)
for n in ['erpin.blend','erpin.fbx','erpin-rig.json']:
 if not (BEFORE/n).exists():shutil.copy2(ASSETS/n,BEFORE/n)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'erpin.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['erpin_SkinnedMesh'];meshes=[o for o in scene.objects if o.type=='MESH'];resolve_character_textures(meshes)
for im in bpy.data.images:
 if im.source=='FILE':
  p=ASSETS/Path(bpy.path.abspath(im.filepath)).name
  if p.is_file():im.filepath=str(p)
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
old={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=581] for c in curves()}
geometry={o.name:[v.co[:] for v in o.data.vertices] for o in meshes}
scene.frame_set(0);base={p.name:p.matrix_basis.copy() for p in rig.pose.bones}
def turn(name,angle):
 p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3();q=base[name].to_quaternion()@(rest.inverted()@Matrix.Rotation(angle,3,'X')@rest).to_quaternion()
 if p.rotation_mode=='QUATERNION':
  if p.rotation_quaternion.dot(q)<0:q=-q
  p.rotation_quaternion=q
 else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)
def aim(name,end,target):
 p=rig.pose.bones[name];d=rig.pose.bones[end].head-p.head;q=d.rotation_difference(target-p.head);p.matrix=Matrix.Translation(p.head)@q.to_matrix().to_4x4()@Matrix.Translation(-p.head)@p.matrix;bpy.context.view_layer.update()
def arm(side,target):
 u=f'UpperArm.{side}';f=f'Forearm.{side}';h=f'Hand.{side}';a=rig.pose.bones[u].head.copy();b=rig.pose.bones[f].head.copy();c=rig.pose.bones[h].head.copy();l1=(b-a).length;l2=(c-b).length;d=target-a;distance=max(abs(l1-l2)+.001,min(l1+l2-.002,d.length));d.normalize();target=a+d*distance
 pole=Vector((-1 if side=='L' else 1,.1,-.8));pole-=d*pole.dot(d);pole.normalize();along=(l1*l1-l2*l2+distance*distance)/(2*distance)
 aim(u,f,a+d*along+pole*math.sqrt(max(0,l1*l1-along*along)));aim(f,h,target)
def smooth(x):x=max(0,min(1,x));return x*x*(3-2*x)
data=json.loads((ASSETS/'skill/erpin.json').read_text());checkpoints=[]
for clip,start in zip(data,[584,672,710,729]):
 count=len(clip['samples'])-1
 for i,s in enumerate(clip['samples']):
  scene.frame_set(start+i)
  for n,m in base.items():rig.pose.bones[n].matrix_basis=m.copy()
  weight=1.0 if clip['clip']=='Ultimate1_2_Loop' else smooth(i/4)*smooth((count-i)/5)
  for n,a in s['rotations'].items():
   if n not in rig.pose.bones or n.startswith(('UpperArm','Forearm','Hand','Foot')):continue
   limit=1.2 if clip['clip']=='Ultimate1_3' else .75
   turn(n,max(-limit,min(limit,-a*.8))*weight)
  root=Vector((0,.30*s['root'][0],.42*s['root'][1]))*weight;hips=rig.pose.bones['Hips'];hips.location=base['Hips'].to_translation()+hips.bone.matrix_local.to_3x3().inverted()@root;bpy.context.view_layer.update()
  for j,side in enumerate(['L','R']):
   p=s['hands'][j];origin=rig.pose.bones[f'Hand.{side}'].head.copy()
   desired=Vector((-.30 if side=='L' else .30,-.22+.40*p[0],.98+.48*p[1]))+root
   arm(side,origin.lerp(desired,weight))
  for p in rig.pose.bones:
   p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=start+i,group=p.name);p.keyframe_insert('location',frame=start+i,group=p.name)
  if i in [0,12,20,58,64,105,count]:checkpoints.append({'clip':clip['clip'],'frame':start+i})
for c in curves():
 for k in c.keyframe_points:
  if k.co.x>=584:k.interpolation='LINEAR'
after={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=581] for c in curves()}
assert all(after[k]==v for k,v in old.items()),'old keys changed'
assert all(geometry[o.name]==[v.co[:] for v in o.data.vertices] for o in meshes),'geometry changed'
# Remove the gray metal remnant on the upper rear scalp only. Reuse boundary
# vertices for the hair cap so existing facial morph coordinate lookup stays valid.
mesh=body.data
img=next(n.image for n in mesh.materials[0].node_tree.nodes if n.type=='TEX_IMAGE' and n.image.name==mesh.materials[0]['meshy_base_texture'])
w,h=img.size;pixels=np.array(img.pixels[:]).reshape(h,w,4)
uv=np.array([l.uv[:] for l in mesh.uv_layers.active.data]);xy=np.clip((uv*np.array([w,h])).astype(int),[0,0],[w-1,h-1])
colors=np.zeros((len(mesh.vertices),3));counts=np.zeros(len(mesh.vertices));indices=np.array([l.vertex_index for l in mesh.loops]);np.add.at(colors,indices,pixels[xy[:,1],xy[:,0],:3]);np.add.at(counts,indices,1);colors/=np.maximum(1,counts[:,None])
points=np.array([v.co[:] for v in mesh.vertices]);r,g,b=colors.T
mask=(points[:,1]>.05)&(points[:,2]>1.35)&(b>r*.90)&(g>r*.95)&(r>.15)
removed=set(int(i) for i in np.flatnonzero(mask));assert 200<len(removed)<500,len(removed)
# UV from adjacent blonde hair, far from eyes/crown/feathers.
hair=next(l.uv.copy() for poly in mesh.polygons for l in [mesh.uv_layers.active.data[poly.loop_start]] if all(points[v,1]>.25 and 1.25<points[v,2]<1.4 and abs(points[v,0])<.12 for v in poly.vertices))
bm=bmesh.new();bm.from_mesh(mesh);bm.verts.ensure_lookup_table()
near=set(v for i in removed for edge in bm.verts[i].link_edges for v in edge.verts if v.index not in removed)
bmesh.ops.delete(bm,geom=[bm.verts[i] for i in removed],context='VERTS')
edges=[e for e in bm.edges if e.is_boundary and all(v in near for v in e.verts)]
# The original source has split boundary seams. Close their envelope with a
# separate Head-skinned hair patch; the original facial vertex lookup stays intact.
from mathutils.geometry import convex_hull_2d
boundary=list({v for e in edges for v in e.verts});outline=[Vector((v.co.x,v.co.z)) for v in boundary]
hull=convex_hull_2d(outline);ring=[boundary[i].co.copy() for i in hull]
center=sum(ring,Vector())/len(ring)
# Fit the adjacent blonde scalp curvature instead of retaining the metal ridge.
local=(points[:,1]>.10)&(points[:,2]>1.32)&(points[:,2]<1.75)&(points[:,0]>-.47)&(points[:,0]<.02)&(~mask)
q=points[local];x=q[:,0];z=q[:,2]-1.5
A=np.column_stack((np.ones(len(q)),x,z,x*x,x*z,z*z));fit=np.linalg.lstsq(A,q[:,1],rcond=None)[0]
x=center.x;z=center.z-1.5;center.y=float(np.dot([1,x,z,x*x,x*z,z*z],fit))+.003
ring=[v+Vector((0,.003,0)) for v in ring];verts=[center]+ring
cap=bpy.data.meshes.new('Erpin repaired rear hair');cap.from_pydata(verts,[],[(0,i+1,(i+1)%len(ring)+1) for i in range(len(ring))]);cap.update()
obj=bpy.data.objects.new('erpin_RearHairRepair',cap);scene.collection.objects.link(obj);obj.matrix_world=body.matrix_world.copy();obj.parent=body.parent
cap.materials.append(mesh.materials[0]);layer=cap.uv_layers.new(name='UVMap')
for loop in layer.data:loop.uv=hair
for poly in cap.polygons:poly.use_smooth=True
vg=obj.vertex_groups.new(name='Head');vg.add(list(range(len(verts))),1.0,'REPLACE');mod=obj.modifiers.new('Armature','ARMATURE');mod.object=rig
filled=[];bm.to_mesh(mesh);bm.free();mesh.update()
print('ERPIN_METAL_REMOVED',len(removed),'filled',len(filled),flush=True)

scene.frame_end=819;scene.frame_set(0);scene.render.fps=30;bpy.context.preferences.filepaths.save_version=0
scene['erpin_skills']='Skill1_1 584:669; Ultimate1_1 672:707; Ultimate1_2_Loop 710:726; Ultimate1_3 729:819'
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'erpin.blend'),relative_remap=False);export_fbx(body,rig,ASSETS/'erpin.fbx')
rig_path=ASSETS/'erpin-rig.json';record=json.loads(rig_path.read_text(encoding='utf-8'));record['vertices']=len(body.data.vertices);record['triangles']=sum(len(p.vertices)-2 for p in body.data.polygons);record['clips'].update(ErpinSkill=[584,669],ErpinStart=[672,707],ErpinCharge=[710,726],ErpinRecover=[729,819]);rig_path.write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
manifest_path=ASSETS/'manifest.json';manifest=json.loads(manifest_path.read_text(encoding='utf-8'));record=next(r for r in manifest if r['slug']=='erpin');record['vertices']=len(body.data.vertices);record['triangles']=sum(len(p.vertices)-2 for p in body.data.polygons);record['clips'].update(ErpinSkill=[584,669],ErpinStart=[672,707],ErpinCharge=[710,726],ErpinRecover=[729,819])
for key in ['blend','model']:record['sha256'][key]=hashlib.sha256((ROOT/record[key]).read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(WORK/'bake-verification.json').write_text(json.dumps({'old_keys_preserved':True,'geometry_preserved_except_rear_metal':True,'removed_rear_vertices':len(removed),'checkpoints':checkpoints},indent=2))
print('ERPIN_SKILLS_BAKE_OK',flush=True)
