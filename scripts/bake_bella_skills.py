"""Append archived Bella E/Q motion, preserving existing meshes, face and old keys."""
import bpy,json,math,sys,shutil,hashlib
from pathlib import Path
from mathutils import Vector,Matrix
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';WORK=ROOT/'artifacts/bella-skills';BEFORE=WORK/'before';BEFORE.mkdir(parents=True,exist_ok=True)
for n in ['bella.blend','bella.fbx','bella-rig.json']:
 if not (BEFORE/n).exists():shutil.copy2(ASSETS/n,BEFORE/n)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'bella.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['bella_SkinnedMesh'];meshes=[o for o in scene.objects if o.type=='MESH'];resolve_character_textures(meshes)
for im in bpy.data.images:
 if im.source=='FILE':
  p=ASSETS/Path(bpy.path.abspath(im.filepath)).name
  if p.is_file():im.filepath=str(p)
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
old={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves()}
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
data=json.loads((ASSETS/'skill/bella.json').read_text());checkpoints=[]
for clip,start in zip(data['clips'],[531,634]):
 count=len(clip['samples'])-1
 for i,s in enumerate(clip['samples']):
  scene.frame_set(start+i)
  for n,m in base.items():rig.pose.bones[n].matrix_basis=m.copy()
  weight=smooth(i/5)*smooth((count-i)/7)
  for n,a in s['rotations'].items():
   if n not in rig.pose.bones or n.startswith(('UpperArm','Forearm','Hand','Foot')):continue
   turn(n,max(-.65,min(.65,-a*.65))*weight)
  root=Vector((0,.30*s['root'][0],.42*s['root'][1]))*weight;hips=rig.pose.bones['Hips'];hips.location=base['Hips'].to_translation()+hips.bone.matrix_local.to_3x3().inverted()@root;bpy.context.view_layer.update()
  for j,side in enumerate(['L','R']):
   p=s['hands'][j];origin=rig.pose.bones[f'Hand.{side}'].head.copy()
   desired=Vector((-.28 if side=='L' else .28,-.25+.32*p[0],1.02+.42*p[1]))+root
   arm(side,origin.lerp(desired,weight))
  for p in rig.pose.bones:
   p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=start+i,group=p.name);p.keyframe_insert('location',frame=start+i,group=p.name)
  if i in [0,12,20,58,64,105,count]:checkpoints.append({'clip':clip['clip'],'frame':start+i})
for c in curves():
 for k in c.keyframe_points:
  if k.co.x>=531:k.interpolation='LINEAR'
after={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves()}
assert all(after[k]==v for k,v in old.items()),'old keys changed'
assert all(geometry[o.name]==[v.co[:] for v in o.data.vertices] for o in meshes),'geometry changed'
scene.frame_end=784;scene.frame_set(0);scene.render.fps=30;bpy.context.preferences.filepaths.save_version=0
scene['bella_skills']='Skill1_1 531:631; Ultimate1_1 634:784; source lantern gesture'
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'bella.blend'),relative_remap=False);export_fbx(body,rig,ASSETS/'bella.fbx')
rig_path=ASSETS/'bella-rig.json';record=json.loads(rig_path.read_text(encoding='utf-8'));record['clips'].update(BellaSkill=[531,631],BellaUltimate=[634,784]);rig_path.write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
manifest_path=ASSETS/'manifest.json';manifest=json.loads(manifest_path.read_text(encoding='utf-8'));record=next(r for r in manifest if r['slug']=='bella');record['clips'].update(BellaSkill=[531,631],BellaUltimate=[634,784])
for key in ['blend','model']:record['sha256'][key]=hashlib.sha256((ROOT/record[key]).read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(WORK/'bake-verification.json').write_text(json.dumps({'old_keys_preserved':True,'geometry_preserved':True,'checkpoints':checkpoints},indent=2))
print('BELLA_BAKE_OK',flush=True)
