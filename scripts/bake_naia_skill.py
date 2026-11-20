"""Bake Skill1_1..4 and append Ultimate1_1 to Naia's existing rig, preserving all original clips/geometry."""
import bpy, json, math, sys
from pathlib import Path
from mathutils import Vector, Matrix, Quaternion
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
from arm_weight_isolation import isolate_refined_arm_weights
ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/naia-ultimate';OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(ASSETS/'naia.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects['naia_SkinnedMesh']
isolate_refined_arm_weights(mesh, rig)
data=json.loads((ASSETS/'skill/naia.json').read_text())
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
def old_keys():return {(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<558] for c in curves()}
before=old_keys();geometry=[v.co[:] for v in mesh.data.vertices]
scene.frame_set(0)
base={p.name:(p.rotation_quaternion.copy() if p.rotation_mode=='QUATERNION' else p.rotation_euler.to_quaternion(),p.location.copy(),p.scale.copy()) for p in rig.pose.bones}
def rotation(p,q):
    if p.rotation_mode=='QUATERNION':
        if p.rotation_quaternion.dot(q)<0:q=-q
        p.rotation_quaternion=q
    else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)
def aim(name,end,target):
    p=rig.pose.bones[name];current=rig.pose.bones[end].head-p.head
    q=current.rotation_difference(target-p.head)
    p.matrix=Matrix.Translation(p.head)@q.to_matrix().to_4x4()@Matrix.Translation(-p.head)@p.matrix
    bpy.context.view_layer.update()
def arm(side,target):
    upper=f'UpperArm.{side}';fore=f'Forearm.{side}';hand=f'Hand.{side}'
    a=rig.pose.bones[upper].head.copy();b=rig.pose.bones[fore].head.copy();c=rig.pose.bones[hand].head.copy()
    l1=(b-a).length;l2=(c-b).length;d=target-a
    distance=max(abs(l1-l2)+.001,min(l1+l2-.002,d.length));d.normalize();target=a+d*distance
    pole=Vector((-1 if side=='L' else 1,-.35,-.75));pole-=d*pole.dot(d);pole.normalize()
    along=(l1*l1-l2*l2+distance*distance)/(2*distance);height=math.sqrt(max(0,l1*l1-along*along))
    aim(upper,fore,a+d*along+pole*height);aim(fore,hand,target)
def smooth(x):x=max(0,min(1,x));return x*x*(3-2*x)
checkpoints=[]
for clip,start in zip(data['clips'],[558,605,632,645,675]):
    samples=clip['samples'];count=len(samples)-1;loop=clip['clip'] in ['Skill1_2','Skill1_3']
    for index,sample in enumerate(samples):
        frame=start+index;scene.frame_set(frame)
        for n,(q,loc,scale) in base.items():p=rig.pose.bones[n];rotation(p,q);p.location=loc;p.scale=scale
        # Keep cycle boundaries continuous while retaining the sampled source accents.
        seam=smooth((index-(count-3))/3) if loop else 0
        val=lambda key,j:sample[key][j]*(1-seam)+samples[0][key][j]*seam
        ultimate=clip['clip']=='Ultimate1_1'
        weight=smooth(index/5) if clip['clip']=='Skill1_1' else smooth((count-index)/6) if clip['clip']=='Skill1_4' else smooth(index/5)*smooth((count-index)/6) if ultimate else 1
        for name,angle in sample['rotations'].items():
            if name.startswith(('UpperArm','Forearm','Hand','Foot')):continue
            angle=angle*(1-seam)+samples[0]['rotations'][name]*seam
            limit=.20 if name=='Hips' else .32 if name in ['Torso','Head'] else .50
            value=max(-limit,min(limit,-angle))*weight
            p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3()
            rotation(p,base[name][0]@(rest.inverted()@Matrix.Rotation(value,3,'X')@rest).to_quaternion())
        root=Vector((0,-.18*val('root',0),.25*val('root',1)))*weight
        hips=rig.pose.bones['Hips'];hips.location=base['Hips'][1]+hips.bone.matrix_local.to_3x3().inverted()@root
        bpy.context.view_layer.update()
        for j,side in enumerate(['L','R']):
            path=[sample['hands'][j][k]*(1-seam)+samples[0]['hands'][j][k]*seam for k in range(3)]
            origin=rig.pose.bones[f'Hand.{side}'].head.copy()
            spread=.22+(.20*math.sin(math.pi*index/count) if ultimate else .0)
            desired=Vector((-spread if side=='L' else spread,-.19+(.45 if ultimate else .30)*path[0],1.10+(.65 if ultimate else .35)*path[1]))+root
            arm(side,origin.lerp(desired,weight))
            for segment in range(3):
                name=f'Braid.{side}.{segment+1:02}';p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3()
                angle=val('hair',j*3+segment)*weight
                q=Quaternion(Vector((1,-.20 if side=='L' else .20,0)).normalized(),max(-.5,min(.5,-angle*.5)))
                rotation(p,(rest.inverted()@q.to_matrix()@rest).to_quaternion())
        bpy.context.view_layer.update()
        evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        floor=min((evaluated.matrix_world@v.co).z for v in evaluated.data.vertices)
        if floor<.003:hips.location+=hips.bone.matrix_local.to_3x3().inverted()@Vector((0,0,.003-floor))
        for p in rig.pose.bones:
            p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=p.name)
            p.keyframe_insert('location',frame=frame,group=p.name)
        if index in [0,count//2,count]:
            bpy.context.view_layer.update()
            checkpoints.append({'clip':clip['clip'],'frame':frame,'bones':{n:list(rig.matrix_world@rig.pose.bones[n].head) for n in ['Hips','Head','Hand.L','Hand.R']}})
for c in curves():
    for k in c.keyframe_points:
        if k.co.x>=558:k.interpolation='LINEAR'
assert all(old_keys()[k]==v for k,v in before.items()),'existing clips changed'
assert geometry==[v.co[:] for v in mesh.data.vertices],'mesh changed'
scene.frame_end=796;scene.frame_set(0);scene.render.fps=30
scene['naia_skill_clips']='Skill1_1 558:602; Skill1_2 605:629; Skill1_3 632:642; Skill1_4 645:672; Ultimate1_1 675:796'
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'naia.blend'),relative_remap=False)
export_fbx(mesh,rig,ASSETS/'naia.fbx')
(OUT/'bake-verification.json').write_text(json.dumps({'original_keys_preserved':True,'geometry_preserved':True,'clips':checkpoints},indent=2))
print('NAIA_SKILL_BAKE_OK',flush=True)
