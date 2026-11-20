"""Bake Butter's three source clips and bind the existing tail; keep old keys and geometry."""
import bpy,json,math,sys,shutil,hashlib
from pathlib import Path
from mathutils import Vector,Matrix
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';WORK=ROOT/'artifacts/butter-skills';BEFORE=WORK/'before';BEFORE.mkdir(parents=True,exist_ok=True)
for n in ['butter.blend','butter.fbx','butter-rig.json']:
 if not (BEFORE/n).exists():shutil.copy2(ASSETS/n,BEFORE/n)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'butter.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['butter_SkinnedMesh'];meshes=[o for o in scene.objects if o.type=='MESH'];resolve_character_textures(meshes)
for im in bpy.data.images:
 if im.source=='FILE':
  p=ASSETS/Path(bpy.path.abspath(im.filepath)).name
  if p.is_file():im.filepath=str(p)
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
old={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves()}
geometry={o.name:[v.co[:] for v in o.data.vertices] for o in meshes}
scene.frame_set(0);base={p.name:p.matrix_basis.copy() for p in rig.pose.bones}
def smooth(x):x=max(0,min(1,x));return x*x*(3-2*x)
# The Meshy tail was fused to the skin and had no Tail group. Select its lower
# rear lobe, excluding all head/hair influences; blend the root into the hips.
tail=body.vertex_groups.get('Tail') or body.vertex_groups.new(name='Tail');weighted=[]
for v in body.data.vertices:
 x,y,z=v.co
 head=sum(g.weight for g in v.groups if body.vertex_groups[g.group].name in ['Head','Hair'])
 if y>.105 and .38<z<.73 and head<.1:
  strength=smooth((y-.105)/.09)*smooth((.73-z)/.07)
  if strength>.001:
   for g in list(v.groups):body.vertex_groups[g.group].add([v.index],g.weight*(1-strength),'REPLACE')
   tail.add([v.index],strength,'REPLACE');weighted.append(v.index)
   influences=sorted([(g.group,g.weight) for g in v.groups if g.weight>1e-6],key=lambda pair:(pair[0]==tail.index,pair[1]),reverse=True)
   keep=influences[:4];total=sum(w for _,w in keep)
   for group,_ in influences[4:]:body.vertex_groups[group].remove([v.index])
   for group,w in keep:body.vertex_groups[group].add([v.index],w/total,'REPLACE')
assert len(weighted)>50,('missing tail',len(weighted))
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
data=json.loads((ASSETS/'skill/butter.json').read_text());checkpoints=[]
for clip,start in zip(data['clips'],[531,579,652]):
 count=len(clip['samples'])-1
 for i,s in enumerate(clip['samples']):
  scene.frame_set(start+i)
  for n,m in base.items():rig.pose.bones[n].matrix_basis=m.copy()
  t=i/30;weight=smooth(i/3)*smooth((count-i)/5);basic=clip['clip']=='Attack1_1';skill=clip['clip']=='Skill1_1'
  release=20/30 if basic else 1.1 if skill else 2.5
  pull=smooth(t/(19/30)) if basic else smooth((t-(.23 if skill else 1.1))/(.55 if skill else 1.2))
  pull*=1-smooth((t-release)/.1)
  recoil=math.sin(math.pi*max(0,min(1,(t-release)/.38))) if t>=release else 0
  for n,a in s['rotations'].items():
   if n.startswith(('UpperArm','Forearm','Hand','Foot')):continue
   limit=.16 if n=='Hips' else .45 if n in ['Torso','Head'] else .50
   turn(n,max(-limit,min(limit,-a*.75))*weight)
  if basic:root=Vector((0,.14*pull-.10*recoil,-.025*pull))
  elif skill:root=Vector((0,.10*pull-.07*recoil,-.025*pull)) # gameplay owns physical lift
  else:root=Vector((0,.23*math.tanh(s['root'][0]),max(-.08,min(.22,s['root'][1]*.1))))*weight
  hips=rig.pose.bones['Hips'];hips.location=base['Hips'].to_translation()+hips.bone.matrix_local.to_3x3().inverted()@root;bpy.context.view_layer.update()
  # Anatomical right is .L in this rig; its hand always carries the ordinary fork.
  right=Vector((-.23,-.28-.05*recoil,.95+.035*pull))+root
  left=Vector((.20,-.12+.18*pull,1.05))+root if pull>.01 else Vector((.31,-.03,.72))+root
  if not basic and not skill and .95<t<3.3:
   right=Vector((-.14,-.13+.18*pull,1.0))+root;left=Vector((.14,-.13+.18*pull,1.0))+root
  for side,target in [('L',right),('R',left)]:arm(side,rig.pose.bones[f'Hand.{side}'].head.lerp(target,weight))
  for p in rig.pose.bones:
   p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=start+i,group=p.name);p.keyframe_insert('location',frame=start+i,group=p.name)
  if i in [0,18,20,30,33,60,75,80,count]:checkpoints.append({'clip':clip['clip'],'frame':start+i,'root':list(root)})
for c in curves():
 for k in c.keyframe_points:
  if k.co.x>=531:k.interpolation='LINEAR'
after={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves()}
assert all(after[k]==v for k,v in old.items()),'old keys changed'
assert all(geometry[o.name]==[v.co[:] for v in o.data.vertices] for o in meshes),'geometry changed'
scene.frame_end=770;scene.frame_set(0);scene.render.fps=30;bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'butter.blend'),relative_remap=False);export_fbx(body,rig,ASSETS/'butter.fbx')
clips={'ButterAttack':[531,576],'ButterSkill':[579,649],'ButterUltimate':[652,770]}
rig_path=ASSETS/'butter-rig.json';record=json.loads(rig_path.read_text(encoding='utf-8'));record['clips'].update(clips);record['tail_vertices']=len(weighted);rig_path.write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
manifest_path=ASSETS/'manifest.json';manifest=json.loads(manifest_path.read_text(encoding='utf-8'));record=next(r for r in manifest if r['slug']=='butter');record['clips'].update(clips)
for key in ['blend','model']:record['sha256'][key]=hashlib.sha256((ROOT/record[key]).read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(WORK/'bake-verification.json').write_text(json.dumps({'old_keys_preserved':True,'geometry_preserved':True,'tail_vertices':len(weighted),'checkpoints':checkpoints},indent=2))
print('BUTTER_BAKE_OK',len(weighted),flush=True)
