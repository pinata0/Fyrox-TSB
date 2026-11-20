"""Append Chloe's form clips, refine Sebastian elbows, retain unrelated authored keys."""
import bpy,math,json,shutil,sys,hashlib
from pathlib import Path
from mathutils import Vector,Matrix,Quaternion
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
from pose_id_rider import world_rotation
from chloe_fixed_arms import solve as fixed_arm, rebuild_sleeves
from pose_chloe_carry import palm_rotation
A=ROOT/'assets/characters';W=ROOT/'artifacts/chloe-skills';B=W/'before';B.mkdir(parents=True,exist_ok=True)
for n in ['chloe.blend','chloe.fbx','chloe-rig.json','chloe.morphs.json']:
 if not (B/n).exists():shutil.copy2(A/n,B/n)
bpy.ops.wm.open_mainfile(filepath=str(B/'chloe.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['chloe_SkinnedMesh'];bear=bpy.data.objects['Sebastian']
meshes=[o for o in scene.objects if o.type=='MESH'];resolve_character_textures(meshes)
for im in bpy.data.images:
 if im.source=='FILE':
  p=A/Path(bpy.path.abspath(im.filepath)).name
  if p.is_file():im.filepath=str(p)
scene.frame_set(0);bpy.context.view_layer.update()
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
saved_keys={(c.data_path,c.array_index):[(tuple(k.co),k.interpolation) for k in c.keyframe_points if not 63<=k.co.x<=120] for c in curves()}
base={p.name:p.matrix_basis.copy() for p in rig.pose.bones};rest={p.name:p.bone.matrix_local.copy() for p in rig.pose.bones}
base_world={p.name:p.matrix.copy() for p in rig.pose.bones}
arm_lengths={s:tuple((rig.pose.bones[f'{b}.{s}'].head-rig.pose.bones[f'{a}.{s}'].head).length for a,b in [('UpperArm','Forearm'),('Forearm','Hand')]) for s in ['L','R']}
rebuild_sleeves(rig)
# Two new deform joints per arm retain every old arm transform outside these clips.
bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
for side,sgn in [('L',-1),('R',1)]:
 shoulder=rig.data.edit_bones[f'Sebastian.Arm.{side}']
 for label,head,tail,parent in [('Elbow',(sgn*.94,.60,.91),(sgn*.99,.63,.30),shoulder.name),('Hand',(sgn*.99,.63,.30),(sgn*.99,.63,.15),f'Sebastian.Elbow.{side}')]:
  b=rig.data.edit_bones.new(f'Sebastian.{label}.{side}');b.head=head;b.tail=tail;b.parent=rig.data.edit_bones[parent]
bpy.ops.object.mode_set(mode='OBJECT')
for side,sgn in [('L',-1),('R',1)]:
 arm=bear.vertex_groups[f'Sebastian.Arm.{side}'];elbow=bear.vertex_groups.new(name=f'Sebastian.Elbow.{side}');hand=bear.vertex_groups.new(name=f'Sebastian.Hand.{side}')
 for v in bear.data.vertices:
  weights={g.group:g.weight for g in v.groups};w=weights.get(arm.index,0.0)
  if w<=0.0:continue
  lower=max(0,min(1,(1.10-v.co.z)/.32));palm=max(0,min(1,(.44-v.co.z)/.18))
  arm.add([v.index],w*(1-lower),'REPLACE');elbow.add([v.index],w*lower*(1-palm),'REPLACE');hand.add([v.index],w*lower*palm,'REPLACE')
# The imported companion's old head only owned its forehead. Isolate the complete
# face and quilted arms so lowering the head cannot shear the cheeks off the skull.
def mask(x):x=max(0,min(1,x));return x*x*(3-2*x)
for v in bear.data.vertices:
 x,y,z=v.co;head=mask((z-1.62)/.19)
 limb=mask((abs(x)-.60)/.22)*mask((y-.13)/.22)*(1-mask((z-1.55)/.20))
 if head>.001 or limb>.001:
  for g in bear.vertex_groups:g.remove([v.index])
  side='L' if x<0 else 'R';lower=mask((1.10-z)/.32);palm=mask((.44-z)/.18);limb*=1-head
  for n,w in [('Sebastian.Head',head),(f'Sebastian.Arm.{side}',limb*(1-lower)),(f'Sebastian.Elbow.{side}',limb*lower*(1-palm)),(f'Sebastian.Hand.{side}',limb*lower*palm),('SebastianRoot',1-head-limb)]:
   if w>.00001:bear.vertex_groups[n].add([v.index],w,'REPLACE')
for p in rig.pose.bones:
 if p.name not in base:base[p.name]=p.matrix_basis.copy();rest[p.name]=p.bone.matrix_local.copy();base_world[p.name]=p.matrix.copy()
profiles=json.loads((A/'chloe-hands.json').read_text())
anchors=json.loads(scene['chloe_carry_pose'])['cheek_anchors']
data=json.loads((A/'skill/chloe.json').read_text());clips={c['clip']:c for c in data['clips']};idle=data['idle'];height=data['height']
def smooth(x):x=max(0,min(1,x));return x*x*(3-2*x)
def restore():
 for n,m in base.items():rig.pose.bones[n].matrix_basis=m.copy()
 bpy.context.view_layer.update()
def shift(name,delta):
 p=rig.pose.bones[name];m=p.matrix.copy();m.translation+=Vector(delta);p.matrix=m;bpy.context.view_layer.update()
def turn(name,angle,axis='X'):
 p=rig.pose.bones[name];m=p.matrix.copy();o=m.translation.copy();p.matrix=Matrix.Translation(o)@Matrix.Rotation(angle,4,axis)@Matrix.Translation(-o)@m;bpy.context.view_layer.update()
def arm(side,target):fixed_arm(rig,side,Vector(target),arm_lengths,base)
def carry():
 # Preserve actual palm contact with the cheek after applying locomotion.
 delta=rig.pose.bones['Sebastian.Head'].matrix@rest['Sebastian.Head'].inverted()
 for p in profiles:
  side=p['side'];rot=delta.to_quaternion()@palm_rotation(p,side);target=delta@Vector(anchors[side]['rest'])+Vector((0,0,-.004));wrist=target-rot@(Vector(p['contact'])-Vector(p['wrist']))
  arm(side,wrist);world_rotation(rig,f'Hand.{side}',rot@rest[f'Hand.{side}'].to_quaternion())
def source_leg(sample,name,strength=1):
 parent='Hips' if name.startswith('Thigh') else 'Thigh.'+name[-1]
 p=sample['bones'];angle=(p[name][2]-idle[name][2]-(p[parent][2]-idle[parent][2])+180)%360-180
 turn(name,max(-.8,min(.8,-math.radians(angle)*strength)))
def bear_arm(side,raise_elbow=0,swing=0,push=0):
 # Keep the shoulder anchored. The lower quilted limb bends at its new elbow.
 sign=-1 if side=='L' else 1
 turn(f'Sebastian.Arm.{side}',-.8*raise_elbow+push*1.15+swing)
 turn(f'Sebastian.Arm.{side}',sign*.80*raise_elbow,'Y')
 turn(f'Sebastian.Elbow.{side}',1.35*raise_elbow-.55*push)
def mounted(t,move=False):
 # Chloe sits astride the upper back; Sebastian no longer inherits her hip lift.
 shift('Hips',(0,.64,1.65))
 rig.pose.bones['SebastianRoot'].matrix=base_world['SebastianRoot'].copy();bpy.context.view_layer.update()
 # Lower the bear head in front of the rider, keeping Chloe's face and aim clear.
 shift('Sebastian.Head',(0,-.42,-.48));turn('Sebastian.Head',-.10)
 for side,sign in [('L',-1),('R',1)]:
  turn(f'Thigh.{side}',-1.18);turn(f'Thigh.{side}',sign*.38,'Y');turn(f'Shin.{side}',1.35)
  if move:
   wave=math.sin(t*math.tau/(20/30)+(0 if side=='L' else math.pi))
   turn(f'Sebastian.Leg.{side}',wave*.30);bear_arm(side,1,wave*.11)
  else:bear_arm(side,.35,math.sin(t*math.tau/(40/30))*.025)
 # Shortened sleeves avoid the old carry arm length becoming an oversized reach.
 for side,sign in [('L',-1),('R',1)]:arm(side,(sign*.42,.15,2.10))
def spread(t):return smooth((t-1.25)/.16)*(1-smooth((t-1.70)/.30))
def pose(name,t,sample):
 restore()
 if name in ['Walk','Run']:
  for side in ['L','R']:
   for b in ['Thigh','Shin']:source_leg(sample,f'{b}.{side}')
  phase=t*math.tau/.8
  shift('Hips',(0,-.04,.025*math.sin(phase*2)))
  # A lagging torso and limp limbs follow Chloe's running feet.
  turn('SebastianRoot',-.10+.035*math.sin(phase));bear_arm('L',0,.09*math.sin(phase-.7));bear_arm('R',0,-.09*math.sin(phase-.7));carry()
 elif name=='ChloeAttack':
  carry();push=smooth((t-.14)/.10)*(1-smooth((t-.33)/.40));bear_arm('L',.2,push=push)
 elif name in ['ChloeRideIdle','ChloeRideMove','ChloeRideAttack','ChloeRoar']:
  mounted(t,name=='ChloeRideMove')
  if name=='ChloeRideMove':
   arm('L',(-.48,-.70,2.45));world_rotation(rig,'Hand.L',Quaternion((1,0,0),math.pi/2)@rest['Hand.L'].to_quaternion())
  if name=='ChloeRideAttack':bear_arm('L',.4,push=smooth((t-.14)/.10)*(1-smooth((t-.33)/.4)))
  if name=='ChloeRoar':
   w=smooth(t/.25)*(1-smooth((t-1.6)/.4));turn('Sebastian.Head',.24*w+math.sin(t*35)*.028*w)
   turn('SebastianRoot',-.16*w);turn('Hips',-.10*w)
   for side,sign in [('L',-1),('R',1)]:
    arm(side,(sign*.29,.62,2.86));bear_arm(side,.85,math.sin(t*25)*.045*w)
 elif name=='ChloeMount':
  mounted(t);target={p.name:p.matrix_basis.copy() for p in rig.pose.bones};w=smooth((t-.25)/.8)
  restore()
  for p in rig.pose.bones:p.matrix_basis=base[p.name].lerp(target[p.name],w)
  bpy.context.view_layer.update()
  if w<.10:carry()
  else:
   for side,sign in [('L',-1),('R',1)]:
    target=Vector((sign*(.42+.65*spread(t)),.15,2.10+.43*spread(t)))
    arm(side,target)
 elif name=='ChloeDismount':
  mounted(t);target={p.name:p.matrix_basis.copy() for p in rig.pose.bones};progress=max(0,min(1,(t-.30)/.95));w=1-smooth(progress)
  restore()
  for p in rig.pose.bones:p.matrix_basis=base[p.name].lerp(target[p.name],w)
  bpy.context.view_layer.update()
  shift('Hips',(0,0,.36*math.sin(progress*math.pi)))
  if w<.10:carry()
def keys(frame):
 for p in rig.pose.bones:
  p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=p.name)
  p.keyframe_insert('location',frame=frame,group=p.name);p.keyframe_insert('scale',frame=frame,group=p.name)
ranges={'ChloeAttack':(531,555,'Skill1_1'),'ChloeMount':(558,618,'Skill1_1'),'ChloeRideIdle':(621,661,'Idle_SebastianForm'),'ChloeRideMove':(664,684,'Move_SebastianForm'),'ChloeRideAttack':(687,711,'Skill1_1'),'ChloeRoar':(714,774,'Skill1_1_SebastianForm'),'ChloeDismount':(777,837,'Skill2_1')}
for name,(start,end,source) in {'Walk':(63,93,'Move'),'Run':(96,120,'Move'),**ranges}.items():
 samples=clips[source]['samples']
 for frame in range(start,end+1):
  scene.frame_set(frame);phase=(frame-start)/(end-start);t=phase*(.8 if name in ['Walk','Run'] else (end-start)/30)
  pose(name,t,samples[min(len(samples)-1,round(phase*(len(samples)-1)))]);keys(frame)
 print('BAKED',name,flush=True)
for c in curves():
 for k in c.keyframe_points:
  if k.co.x>=531 or 63<=k.co.x<=120:k.interpolation='LINEAR'
for c in curves():
 key=(c.data_path,c.array_index)
 if key in saved_keys:
  now=[(tuple(k.co),k.interpolation) for k in c.keyframe_points if k.co.x<=528 and not 63<=k.co.x<=120]
  assert now==saved_keys[key],('unrelated keys changed',key)
scene.frame_end=837;scene.frame_set(0);scene.render.fps=30;bpy.context.preferences.filepaths.save_version=0
from chloe_face_meshes import build_faces
build_faces(rig,body,bear)
bpy.ops.wm.save_as_mainfile(filepath=str(A/'chloe.blend'),relative_remap=False)
export_fbx(body,rig,A/'chloe.fbx',bake_step=.5)
rp=A/'chloe-rig.json';record=json.loads(rp.read_text(encoding='utf8'));record['clips'].update({n:[v[0],v[1]] for n,v in ranges.items()});record['bones']=len(rig.data.bones);rp.write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
mp=A/'manifest.json';records=json.loads(mp.read_text(encoding='utf8'));r=next(r for r in records if r['slug']=='chloe');r['clips'].update(record['clips'])
for key in ['blend','model']:r['sha256'][key]=hashlib.sha256((ROOT/r[key]).read_bytes()).hexdigest()
mp.write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
(W/'bake-verification.json').write_text(json.dumps({'unrelated_keys_preserved':True,'bones':len(rig.data.bones),'clips':ranges},indent=2))
print('CHLOE_BAKE_OK',flush=True)
