"""Retarget four archived clips and add cloth chains; preserve face/geometry/old keys."""
import bpy,json,math,sys,shutil
from pathlib import Path
from mathutils import Vector,Matrix,Quaternion
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';WORK=ROOT/'artifacts/renua-skills';BEFORE=WORK/'before';BEFORE.mkdir(parents=True,exist_ok=True)
for n in ['renua.blend','renua.fbx','renua-rig.json']:
    if not (BEFORE/n).exists():shutil.copy2(ASSETS/n,BEFORE/n)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'renua.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['renua_SkinnedMesh'];coat=bpy.data.objects['renua_RestoredCoat']
meshes=[o for o in scene.objects if o.type=='MESH'];resolve_character_textures(meshes)
for im in bpy.data.images:
    if im.source=='FILE':
        p=ASSETS/Path(bpy.path.abspath(im.filepath)).name
        if p.is_file():im.filepath=str(p)
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
old={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves()}
geometry={o.name:[v.co[:] for v in o.data.vertices] for o in meshes}
scene.frame_set(0)
base={p.name:p.matrix_basis.copy() for p in rig.pose.bones}
bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
for side,x in [('L',-.31),('C',0),('R',.31)]:
    for k in range(3):
        b=rig.data.edit_bones.new(f'RenuaCoat.{side}.{k}');b.head=(x,.13,.86-k*.18);b.tail=(x,.13,.68-k*.18)
        b.parent=rig.data.edit_bones['Torso' if k==0 else f'RenuaCoat.{side}.{k-1}']
for side,x in [('L',-.23),('R',.23)]:
    for k in range(2):
        b=rig.data.edit_bones.new(f'RenuaHair.{side}.{k}');b.head=(x,.12,1.12-k*.33);b.tail=(x,.12,.79-k*.33)
        b.parent=rig.data.edit_bones['Head' if k==0 else f'RenuaHair.{side}.{k-1}']
bpy.ops.object.mode_set(mode='OBJECT')
new=[p.name for p in rig.pose.bones if p.name.startswith(('RenuaCoat','RenuaHair'))]
for n in new:base[n]=Matrix.Identity(4)
groups={n:coat.vertex_groups.new(name=n) for n in new if n.startswith('RenuaCoat')}
weighted=0
for v in coat.data.vertices:
    # Sleeves/shoulders retain their original arm weights. Only the free hem flutters.
    blend=max(0,min(1,(.78-v.co.z)/.18))
    if blend<=0:continue
    existing=[(g.group,g.weight) for g in v.groups]
    for index,w in existing:coat.vertex_groups[index].add([v.index],w*(1-blend),'REPLACE')
    side='C' if abs(v.co.x)<.15 else 'L' if v.co.x<0 else 'R'
    step=max(0,min(2,(.75-v.co.z)/.18));lo=int(step);hi=min(2,lo+1);f=step-lo
    groups[f'RenuaCoat.{side}.{lo}'].add([v.index],blend*(1-f),'REPLACE')
    if hi!=lo:groups[f'RenuaCoat.{side}.{hi}'].add([v.index],blend*f,'REPLACE')
    weighted+=1
hair=body.vertex_groups.get('Hair');hair_index=hair.index if hair else -1
hgroups={n:body.vertex_groups.new(name=n) for n in new if n.startswith('RenuaHair')}
for v in body.data.vertices:
    w=next((g.weight for g in v.groups if g.group==hair_index),0);f=max(0,min(1,(1.12-v.co.z)/.22))
    if w*f<=0:continue
    hair.add([v.index],w*(1-f),'REPLACE');side='L' if v.co.x<0 else 'R';step=max(0,min(1,(1.0-v.co.z)/.33))
    hgroups[f'RenuaHair.{side}.0'].add([v.index],w*f*(1-step),'REPLACE');hgroups[f'RenuaHair.{side}.1'].add([v.index],w*f*step,'REPLACE')
def rotation(p,q):
    if p.rotation_mode=='QUATERNION':
        if p.rotation_quaternion.dot(q)<0:q=-q
        p.rotation_quaternion=q
    else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)
def turn(name,angle,axis='X'):
    p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3()
    rotation(p,base[name].to_quaternion()@(rest.inverted()@Matrix.Rotation(angle,3,axis)@rest).to_quaternion())
def aim(name,end,target):
    p=rig.pose.bones[name];current=rig.pose.bones[end].head-p.head;q=current.rotation_difference(target-p.head)
    p.matrix=Matrix.Translation(p.head)@q.to_matrix().to_4x4()@Matrix.Translation(-p.head)@p.matrix;bpy.context.view_layer.update()
def arm(side,target):
    upper=f'UpperArm.{side}';fore=f'Forearm.{side}';hand=f'Hand.{side}'
    a=rig.pose.bones[upper].head.copy();b=rig.pose.bones[fore].head.copy();c=rig.pose.bones[hand].head.copy();l1=(b-a).length;l2=(c-b).length;d=target-a
    distance=max(abs(l1-l2)+.001,min(l1+l2-.002,d.length));d.normalize();target=a+d*distance
    pole=Vector((-1 if side=='L' else 1,.1,-.8));pole-=d*pole.dot(d);pole.normalize();along=(l1*l1-l2*l2+distance*distance)/(2*distance)
    aim(upper,fore,a+d*along+pole*math.sqrt(max(0,l1*l1-along*along)));aim(fore,hand,target)
def smooth(x):x=max(0,min(1,x));return x*x*(3-2*x)
data=json.loads((ASSETS/'skill/renua.json').read_text());checkpoints=[]
for clip,start in zip(data['clips'][1:],[531,615,704,856]):
    samples=clip['samples'];count=len(samples)-1
    for i,s in enumerate(samples):
        scene.frame_set(start+i)
        for n,m in base.items():rig.pose.bones[n].matrix_basis=m.copy()
        weight=smooth(i/5)*smooth((count-i)/7)
        for n,a in s['rotations'].items():
            if n.startswith(('UpperArm','Forearm','Hand','Foot')):continue
            turn(n,max(-.65,min(.65,-a*.65))*weight)
        root=Vector((0,.30*s['root'][0],.42*s['root'][1]))*weight
        hips=rig.pose.bones['Hips'];hips.location=base['Hips'].to_translation()+hips.bone.matrix_local.to_3x3().inverted()@root
        bpy.context.view_layer.update()
        for j,side in enumerate(['L','R']):
            p=s['hands'][j];origin=rig.pose.bones[f'Hand.{side}'].head.copy()
            spread=.32 if clip['clip'] in ['Skill1_1','Ultimate1_1'] else .24
            desired=Vector((-spread if side=='L' else spread,-.19+.40*p[0],.98+.48*p[1]))+root
            arm(side,origin.lerp(desired,weight))
            for k in range(2):turn(f'RenuaHair.{side}.{k}',max(-.65,min(.65,-s['details'][10+j*2+k]*.65))*weight)
        for j,side in enumerate(['L','C','R']):
            for k in range(3):
                # The source's free back panel and side chains are sampled separately.
                index=3+k if side=='C' else (6 if side=='L' else 8)+min(k,1)
                turn(f'RenuaCoat.{side}.{k}',max(-.70,min(.70,-s['details'][index]*.65))*weight)
        for p in rig.pose.bones:
            p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=start+i,group=p.name)
            p.keyframe_insert('location',frame=start+i,group=p.name)
        if i in [0,count//2,count]:checkpoints.append({'clip':clip['clip'],'frame':start+i,'root':list(root)})
# New chains remain neutral in every previously delivered clip, including selection.
for n in new:
    p=rig.pose.bones[n];p.matrix_basis=base[n]
    for frame in [0,528]:
        p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=n);p.keyframe_insert('location',frame=frame,group=n)
for c in curves():
    for k in c.keyframe_points:
        if k.co.x>=531:k.interpolation='LINEAR'
after={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves()}
assert all(after[k]==v for k,v in old.items()),'old animation keys changed'
assert all(geometry[o.name]==[v.co[:] for v in o.data.vertices] for o in meshes),'original geometry changed'
assert weighted>100
scene.frame_end=1337;scene.frame_set(0);scene.render.fps=30;bpy.context.preferences.filepaths.save_version=0
scene['renua_skills']='Attack1_1 531:612; Attack2_1 615:701; Skill1_1 704:853; Ultimate1_1 856:1337'
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'renua.blend'),relative_remap=False)
export_fbx(body,rig,ASSETS/'renua.fbx')
metadata=json.loads((ASSETS/'renua-rig.json').read_text(encoding='utf-8'))
metadata['bones']=len(rig.data.bones)
metadata['clips'].update({'RenuaBasic':[531,612],'RenuaEnhanced':[615,701],'RenuaSkill':[704,853],'RenuaUltimate':[856,1337]})
metadata['renua_skills']={'source':'renua-motion-reference.skel','cloth_bones':new,'samples':'skill/renua.json'}
(ASSETS/'renua-rig.json').write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(WORK/'bake-verification.json').write_text(json.dumps({'old_keys_preserved':True,'geometry_preserved':True,'cloth_vertices':weighted,'new_bones':new,'checkpoints':checkpoints},indent=2))
print('RENUA_BAKE_OK',weighted,len(new),flush=True)
