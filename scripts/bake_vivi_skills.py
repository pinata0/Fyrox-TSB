"""Retarget Vivi's archived motion; add independently weighted wing and skirt chains."""
import bpy,json,math,sys,shutil
from pathlib import Path
from mathutils import Vector,Matrix,Quaternion
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';WORK=ROOT/'artifacts/vivi-skills';BEFORE=WORK/'before';BEFORE.mkdir(parents=True,exist_ok=True)
for n in ['vivi.blend','vivi.fbx','vivi-rig.json']:
 if not (BEFORE/n).exists():shutil.copy2(ASSETS/n,BEFORE/n)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'vivi.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['vivi_SkinnedMesh'];meshes=[o for o in scene.objects if o.type=='MESH'];resolve_character_textures(meshes)
for im in bpy.data.images:
 if im.source=='FILE':
  p=ASSETS/Path(bpy.path.abspath(im.filepath)).name
  if p.is_file():im.filepath=str(p)
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
old={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if 63<=k.co.x<=555] for c in curves()}
geometry={o.name:[v.co[:] for v in o.data.vertices] for o in meshes};scene.frame_set(0);base={p.name:p.matrix_basis.copy() for p in rig.pose.bones}
bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
for side,sign in [('L',-1),('R',1)]:
 for j in range(2):
  b=rig.data.edit_bones.new(f'ViviWing.{side}.{j}');b.head=(sign*(.28+j*.30),.27,1.05+j*.08);b.tail=(sign*(.58+j*.27),.29,1.13+j*.08);b.parent=rig.data.edit_bones['Torso' if j==0 else f'ViviWing.{side}.0']
 for j in range(3):
  b=rig.data.edit_bones.new(f'ViviSkirt.{side}.{j}');b.head=(sign*.22,-.06,.68-j*.18);b.tail=(sign*.26,-.10,.50-j*.18);b.parent=rig.data.edit_bones['Hips' if j==0 else f'ViviSkirt.{side}.{j-1}']
bpy.ops.object.mode_set(mode='OBJECT');new=[p.name for p in rig.pose.bones if p.name.startswith('Vivi')]
for n in new:base[n]=Matrix.Identity(4)
groups={n:body.vertex_groups.new(name=n) for n in new}
def smooth(x):x=max(0,min(1,x));return x*x*(3-2*x)
counts={'wings':0,'skirt':0}
for v in body.data.vertices:
 x,y,z=v.co;side='L' if x<0 else 'R'
 # Outer wing membranes lie behind the shoulder and above the long hanging hair.
 w=smooth((abs(x)-.29)/.17)*smooth((z-.91)/.10)*smooth((y-.15)/.13)*(1-smooth((z-1.48)/.05))
 skirt=smooth((.70-z)/.18)*smooth((.11-y)/.15)*smooth((.47-abs(x))/.12)*smooth((z-.10)/.08)
 if w>.001:
  family='ViviWing';part=smooth((abs(x)-.51)/.24);assign=[(0,1-part),(1,part)];amount=w;counts['wings']+=1
 elif skirt>.001:
  family='ViviSkirt';part=max(0,min(2,(.62-z)/.18));lo=int(part);hi=min(2,lo+1);f=part-lo;assign=[(lo,1-f)] if lo==hi else [(lo,1-f),(hi,f)];amount=skirt;counts['skirt']+=1
 else:continue
 for g in list(v.groups):body.vertex_groups[g.group].add([v.index],g.weight*(1-amount),'REPLACE')
 for j,f in assign:
  if f*amount>0:groups[f'{family}.{side}.{j}'].add([v.index],f*amount,'REPLACE')
def rotation(p,q):
 if p.rotation_mode=='QUATERNION':
  if p.rotation_quaternion.dot(q)<0:q=-q
  p.rotation_quaternion=q
 else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)
def turn(name,angle,axis='X'):
 p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3();rotation(p,base[name].to_quaternion()@(rest.inverted()@Matrix.Rotation(angle,3,axis)@rest).to_quaternion())
def aim(name,end,target):
 p=rig.pose.bones[name];current=rig.pose.bones[end].head-p.head;q=current.rotation_difference(target-p.head);p.matrix=Matrix.Translation(p.head)@q.to_matrix().to_4x4()@Matrix.Translation(-p.head)@p.matrix;bpy.context.view_layer.update()
def arm(side,target):
 upper=f'UpperArm.{side}';fore=f'Forearm.{side}';hand=f'Hand.{side}';a=rig.pose.bones[upper].head.copy();b=rig.pose.bones[fore].head.copy();c=rig.pose.bones[hand].head.copy();l1=(b-a).length;l2=(c-b).length;d=target-a
 distance=max(abs(l1-l2)+.001,min(l1+l2-.002,d.length));d.normalize();target=a+d*distance
 pole=Vector((-1 if side=='L' else 1,-.35,-.8));pole-=d*pole.dot(d);pole.normalize();along=(l1*l1-l2*l2+distance*distance)/(2*distance)
 aim(upper,fore,a+d*along+pole*math.sqrt(max(0,l1*l1-along*along)));aim(fore,hand,target)
def path(knots,t):
 for (a,p),(b,q) in zip(knots,knots[1:]):
  if t<=b:return Vector(p).lerp(Vector(q),smooth((t-a)/(b-a)))
 return Vector(knots[-1][1])
def skill_lift(t):return .42*smooth((t-.52)/.18)*(1-smooth((t-1.03)/.43))
data=json.loads((ASSETS/'skill/vivi.json').read_text());clips=data['clips']
jobs=[('Idle',0,60,0,0,40),('ViviLeft',558,24,1,0,24),('ViviRight',585,33,1,24,57),('ViviSkill',621,44,2,0,44),('ViviUltimate',668,120,3,0,120)];checkpoints=[]
for name,start,count,clip,lo,hi in jobs:
 samples=clips[clip]['samples']
 for i in range(count+1):
  f=lo+(hi-lo)*i/count;idx=int(f);mix=f-idx;s={k:None for k in []}
  sa=samples[idx];sb=samples[min(idx+1,len(samples)-1)]
  s['rotations']={n:a+(sb['rotations'][n]-a)*mix for n,a in sa['rotations'].items()}
  for k in ['root','details']:s[k]=[a+(b-a)*mix for a,b in zip(sa[k],sb[k])]
  s['hands']=[[a+(b-a)*mix for a,b in zip(pa,pb)] for pa,pb in zip(sa['hands'],sb['hands'])]
  scene.frame_set(start+i)
  for n,m in base.items():rig.pose.bones[n].matrix_basis=m.copy()
  t=i/30;weight=1 if name=='Idle' else smooth(i/3)*smooth((count-i)/5)
  for n,a in s['rotations'].items():
   if n.startswith(('UpperArm','Forearm','Hand','Foot')):continue
   turn(n,max(-.40,min(.40,-a*.42))*weight)
  root=Vector((0,0,skill_lift(t) if name=='ViviSkill' else .02*math.sin(i/count*math.tau) if name=='Idle' else 0))
  hips=rig.pose.bones['Hips'];hips.location=base['Hips'].to_translation()+hips.bone.matrix_local.to_3x3().inverted()@root;bpy.context.view_layer.update()
  for j,side in enumerate(['L','R']):
   sign=-1 if side=='L' else 1;origin=rig.pose.bones[f'Hand.{side}'].head.copy();p=s['hands'][j]
   desired=Vector((sign*.22,-.16-.18*p[0],.94+.26*p[1]))+root
   if name in ['ViviLeft','ViviRight']:
    reverse=1 if name=='ViviLeft' else -1;sweep=reverse*(1-2*smooth(t/(count/30*.7)))
    desired=Vector((sign*.14+sweep*.16,-.22,.98+.10*math.sin(t/(count/30)*math.pi)))+root
   # The imported rig uses coordinate names: Hand.L is the anatomical right hand.
   if name=='ViviSkill' and side=='L':
    desired=path([(0,origin),(.28,(.11,-.17,1.08)),(.55,(.11,-.17,1.08)),(.75,(-.10,-.14,1.28)),(1.03,(-.10,-.14,1.28)),(44/30,origin)],t)+root
   if name=='ViviUltimate':
    desired=path([(0,origin),(.32,(sign*.19,-.17,1.14)),(.75,(sign*.15,-.16,1.26)),(1.13,(sign*.17,-.23,1.19)),(1.35,(sign*.18,-.24,.95)),(2.1,(sign*.22,-.18,1.12)),(4,origin)],t)
   if name!='Idle':arm(side,origin.lerp(desired,weight))
   # Original wing tracks supply the bounce, phase lag and elastic tip fold.
   wing=s['details'][j*3];tip=s['details'][j*3+1]
   flap=max(-.56,min(.56,wing*.58))
   if name=='ViviSkill':flap=path([(0,(0,0,0)),(.18,(-.42,0,0)),(.38,(.27,0,0)),(.55,(-.35,0,0)),(.72,(.42,0,0)),(1.05,(.42,0,0)),(44/30,(0,0,0))],t).x
   turn(f'ViviWing.{side}.0',-sign*flap,'Y');turn(f'ViviWing.{side}.1',-sign*max(-.3,min(.3,tip*.28)),'Y')
   for k in range(3):
    flutter=max(-.26,min(.26,s['details'][6+j*3+k]*.55))*weight
    if name=='ViviSkill':flutter+=(.08+math.sin(t*18+k*.7)*.10)*smooth((t-.45)/.2)*smooth((44/30-t)/.3)
    if name=='ViviUltimate':flutter+=math.sin(t*14+k*.9)*.08*weight
    turn(f'ViviSkirt.{side}.{k}',flutter,'X')
   if name!='Idle':turn('Hair',.08*math.sin(t*8)*weight)
  for p in rig.pose.bones:
   p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=start+i,group=p.name);p.keyframe_insert('location',frame=start+i,group=p.name)
  if i in [0,count//2,count]:checkpoints.append({'clip':name,'frame':start+i,'root':list(root)})
# The new wing bones also flap throughout locomotion/selection. Existing bone keys stay intact.
for frame in range(63,556):
 scene.frame_set(frame)
 for j,side in enumerate(['L','R']):
  sign=-1 if side=='L' else 1
  for k in range(2):
   n=f'ViviWing.{side}.{k}';p=rig.pose.bones[n];p.matrix_basis=Matrix.Identity(4);turn(n,-sign*(.24 if k==0 else .10)*math.sin((frame-63)/40*math.tau-k*.5),'Y')
   p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=n);p.keyframe_insert('location',frame=frame,group=n)
  for k in range(3):
   n=f'ViviSkirt.{side}.{k}';p=rig.pose.bones[n];p.matrix_basis=Matrix.Identity(4)
   p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=n);p.keyframe_insert('location',frame=frame,group=n)
for c in curves():
 for k in c.keyframe_points:
  if k.co.x>=558 or k.co.x<=60 or 'Vivi' in c.data_path:k.interpolation='LINEAR'
after={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if 63<=k.co.x<=555] for c in curves()}
assert all(after[k]==v for k,v in old.items()),'previous non-idle keys changed'
assert all(geometry[o.name]==[v.co[:] for v in o.data.vertices] for o in meshes),'geometry changed'
assert counts['wings']>300 and counts['skirt']>300
scene.frame_end=788;scene.frame_set(0);scene.render.fps=30;bpy.context.preferences.filepaths.save_version=0
scene['vivi_skills']='Idle 0:60; ViviLeft 558:582; ViviRight 585:618; ViviSkill 621:665; ViviUltimate 668:788; Q lift is authoritative actor movement'
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'vivi.blend'),relative_remap=False);export_fbx(body,rig,ASSETS/'vivi.fbx')
metadata=json.loads((BEFORE/'vivi-rig.json').read_text(encoding='utf8'));metadata['bones']=len(rig.data.bones);metadata['clips'].update({n:[st,st+co] for n,st,co,*_ in jobs});metadata['vivi_skills']={'source':'skill/vivi.json','new_bones':new,'weighted_vertices':counts}
(ASSETS/'vivi-rig.json').write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+'\n',encoding='utf8');(WORK/'bake-verification.json').write_text(json.dumps({'old_non_idle_keys_preserved':True,'geometry_preserved':True,'weighted_vertices':counts,'checkpoints':checkpoints},indent=2))
print('VIVI_BAKE_OK',counts,flush=True)
