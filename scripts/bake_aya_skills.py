"""Retarget Aya Skill1_1/Ultimate1_1 and skin eight two-link skirt panels.

Preserves base geometry, facial features, materials and keys through frame 555.
Rebuilds from the non-overwritten baseline so repeated runs are deterministic.
"""
import bpy,json,math,sys,shutil,hashlib
from pathlib import Path
from mathutils import Vector,Matrix
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';WORK=ROOT/'artifacts/aya-skills';BEFORE=WORK/'before';BEFORE.mkdir(parents=True,exist_ok=True)
for n in ['aya.blend','aya.fbx','aya-rig.json']:
    if not (BEFORE/n).exists():shutil.copy2(ASSETS/n,BEFORE/n)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'aya.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['aya_SkinnedMesh']
meshes=[o for o in scene.objects if o.type=='MESH'];resolve_character_textures(meshes)
for im in bpy.data.images:
    if im.source=='FILE':
        p=ASSETS/Path(bpy.path.abspath(im.filepath)).name
        if p.is_file():im.filepath=str(p)
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
old={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=555] for c in curves()}
geometry={o.name:[v.co[:] for v in o.data.vertices] for o in meshes}
scene.frame_set(0);base={p.name:p.matrix_basis.copy() for p in rig.pose.bones}
bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
for j in range(8):
    angle=j*math.tau/8
    for k in range(2):
        b=rig.data.edit_bones.new(f'AyaSkirt.{j}.{k}')
        b.head=(.22*math.cos(angle),.17*math.sin(angle),.91-k*.30)
        b.tail=(.32*math.cos(angle),.24*math.sin(angle),.61-k*.30)
        b.parent=rig.data.edit_bones['Hips' if k==0 else f'AyaSkirt.{j}.0']
bpy.ops.object.mode_set(mode='OBJECT')
new=[p.name for p in rig.pose.bones if p.name.startswith('AyaSkirt.')]
for n in new:base[n]=Matrix.Identity(4)
groups={n:body.vertex_groups.new(name=n) for n in new};weighted=0
def smooth(t):t=max(0,min(1,t));return t*t*(3-2*t)
for v in body.data.vertices:
    x,y,z=v.co
    if not .19<z<.91:continue
    # Exclude twin tails, sleeves, hands and the inner legs. The free skirt was
    # previously weighted to the thighs, making it look like a rigid bell.
    cloth=sum(g.weight for g in v.groups if body.vertex_groups[g.group].name.startswith(('Hips','Thigh','Shin','Torso')))
    radius=math.sqrt((x/.43)**2+(y/.29)**2)
    blend=cloth*smooth((.91-z)/.16)*smooth((radius-.40)/.20)
    if blend<.005:continue
    existing=[(g.group,g.weight) for g in v.groups]
    for index,w in existing:body.vertex_groups[index].add([v.index],w*(1-blend),'REPLACE')
    angle=(math.atan2(y/.29,x/.43)%math.tau)/math.tau*8;j=int(angle);f=angle-j
    lower=smooth((.64-z)/.24)
    for sector,weight in [(j,1-f),((j+1)%8,f)]:
        for k,w in [(0,1-lower),(1,lower)]:
            if weight*w>0:groups[f'AyaSkirt.{sector}.{k}'].add([v.index],blend*weight*w,'REPLACE')
    weighted+=1
def rotation(p,q):
    if p.rotation_mode=='QUATERNION':
        if p.rotation_quaternion.dot(q)<0:q=-q
        p.rotation_quaternion=q
    else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)
def turn(name,angle,axis='X'):
    p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3()
    rotation(p,base[name].to_quaternion()@(rest.inverted()@Matrix.Rotation(angle,3,axis)@rest).to_quaternion())
def aim(name,end,target):
    p=rig.pose.bones[name];q=(rig.pose.bones[end].head-p.head).rotation_difference(target-p.head)
    p.matrix=Matrix.Translation(p.head)@q.to_matrix().to_4x4()@Matrix.Translation(-p.head)@p.matrix;bpy.context.view_layer.update()
def arm(side,target):
    upper=f'UpperArm.{side}';fore=f'Forearm.{side}';hand=f'Hand.{side}'
    a=rig.pose.bones[upper].head.copy();b=rig.pose.bones[fore].head.copy();c=rig.pose.bones[hand].head.copy();l1=(b-a).length;l2=(c-b).length;d=target-a
    distance=max(abs(l1-l2)+.001,min(l1+l2-.002,d.length));d.normalize();target=a+d*distance
    pole=Vector((-1 if side=='L' else 1,-.15,-.8));pole-=d*pole.dot(d);pole.normalize();along=(l1*l1-l2*l2+distance*distance)/(2*distance)
    aim(upper,fore,a+d*along+pole*math.sqrt(max(0,l1*l1-along*along)));aim(fore,hand,target)
data=json.loads((ASSETS/'skill/aya.json').read_text());checkpoints=[]
for clip,start in zip(data['clips'],[558,655]):
    samples=clip['samples'];count=len(samples)-1;previous={}
    for i,s in enumerate(samples):
        scene.frame_set(start+i)
        for n,m in base.items():rig.pose.bones[n].matrix_basis=m.copy()
        weight=smooth(i/5)*smooth((count-i)/8);age=i/30
        for n,a in s['rotations'].items():
            if n.startswith(('UpperArm','Forearm','Hand','Foot')):continue
            limit=.28 if n=='Hips' else .48
            turn(n,max(-limit,min(limit,-a*.60))*weight)
        root=Vector((0,-.16*math.tanh(s['root'][0]),max(-.05,min(.33,s['root'][1]*.55))))*weight
        hips=rig.pose.bones['Hips'];hips.location=base['Hips'].to_translation()+hips.bone.matrix_local.to_3x3().inverted()@root
        bpy.context.view_layer.update()
        spread=smooth((age-1.65)/.25)*smooth((3.48-age)/.65) if clip['clip']=='Ultimate1_1' else 0
        for j,side in enumerate(['L','R']):
            p=s['hands'][j];origin=rig.pose.bones[f'Hand.{side}'].head.copy();sign=-1 if side=='L' else 1
            desired=Vector((sign*(.33+.28*spread),-.13+.25*max(-.65,min(.65,p[0])),.96+.38*max(-.45,min(.7,p[1]))+.13*spread))+root
            desired.z=desired.z*(1-spread)+(rig.pose.bones[f'UpperArm.{side}'].head.z+.025)*spread
            arm(side,origin.lerp(desired,weight))
        for j in range(8):
            sector=j//2;angle=j*math.tau/8
            for k in range(2):
                lag=samples[max(0,i-k*3)]['details'];index=1+sector*4+k*2
                a=max(-.34,min(.34,-lag[index]*.65-lag[index+1]*.25))*weight
                # Radial panel hinge, with delayed tips and the original skirt-root twist.
                turn(f'AyaSkirt.{j}.{k}',a,Vector((-math.sin(angle),math.cos(angle),0)))
        for p in rig.pose.bones:
            if p.rotation_mode!='QUATERNION' and p.name in previous:p.rotation_euler.make_compatible(previous[p.name])
            previous[p.name]=p.rotation_euler.copy()
            p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=start+i,group=p.name)
            p.keyframe_insert('location',frame=start+i,group=p.name)
        if i in [0,15,39,57,72,count]:checkpoints.append({'clip':clip['clip'],'frame':start+i,'root':list(root),'hands':[list(rig.pose.bones[f'Hand.{s}'].head) for s in ['L','R']]})
for n in new:
    p=rig.pose.bones[n];p.matrix_basis=base[n]
    for frame in [0,555]:
        p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=n);p.keyframe_insert('location',frame=frame,group=n)
for c in curves():
    for k in c.keyframe_points:
        if k.co.x>=558:k.interpolation='LINEAR'
after={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=555] for c in curves()}
assert all(after[k]==v for k,v in old.items()),'old animation keys changed'
assert all(geometry[o.name]==[v.co[:] for v in o.data.vertices] for o in meshes),'base geometry changed'
assert weighted>500
scene.frame_end=765;scene.frame_set(0);scene.render.fps=30;bpy.context.preferences.filepaths.save_version=0
scene['aya_skills']='Skill1_1 558:652; Ultimate1_1 655:765; frost release native frame 57'
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'aya.blend'),relative_remap=False)
export_fbx(body,rig,ASSETS/'aya.fbx')
rig_path=ASSETS/'aya-rig.json';record=json.loads(rig_path.read_text());record['clips'].update(AyaSkill=[558,652],AyaUltimate=[655,765]);record['skirt_bones']=new;rig_path.write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
manifest_path=ASSETS/'manifest.json';manifest=json.loads(manifest_path.read_text(encoding='utf-8'));record=next(r for r in manifest if r['slug']=='aya');record['clips'].update(AyaSkill=[558,652],AyaUltimate=[655,765])
for key in ['blend','model']:record['sha256'][key]=hashlib.sha256((ROOT/record[key]).read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(WORK/'bake-verification.json').write_text(json.dumps({'old_keys_preserved':True,'geometry_preserved':True,'cloth_vertices':weighted,'new_bones':new,'checkpoints':checkpoints},indent=2))
print('AYA_SKILL_BAKE_OK',weighted,len(new),flush=True)
