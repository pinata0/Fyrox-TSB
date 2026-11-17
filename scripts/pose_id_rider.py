"""Seat Id inside the crescent, baking all existing 30 Hz clips.

Builds artifacts/id-seated-pose/candidate from a once-only production backup.
The body geometry, face keys, skin weights and mount geometry are unchanged.
"""
import bpy, json, math, shutil, sys, hashlib
import numpy as np
from pathlib import Path
from mathutils import Vector, Quaternion, Matrix

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx, resolve_character_textures
import resize_id_mount as review
WORK=ROOT/'artifacts/id-seated-pose'
BEFORE=WORK/'before'
OUT=WORK/'candidate'
ASSETS=ROOT/'assets/characters'
HIP_OFFSET=Vector((0,.42,-.135))
LEG_ANGLES={'Thigh':-2.17,'Shin':.95,'Foot':1.10}
REST_ARMS={'Idle','Walk','Run','Jump'}

def world_rotation(rig,name,rotation):
    p=rig.pose.bones[name]
    matrix=rotation.to_matrix().to_4x4()
    matrix.translation=p.matrix.translation
    basis=rig.convert_space(pose_bone=p,matrix=matrix,from_space='POSE',to_space='LOCAL')
    p.rotation_mode='QUATERNION'
    p.rotation_quaternion=basis.to_quaternion()
    bpy.context.view_layer.update()

def arm_ik(rig,side,target):
    names=[f'{part}.{side}' for part in ['UpperArm','Forearm','Hand']]
    shoulder,elbow,wrist=[rig.pose.bones[n].head.copy() for n in names]
    a=(elbow-shoulder).length;b=(wrist-elbow).length
    direction=target-shoulder;distance=min(direction.length,a+b-.002)
    direction.normalize()
    pole=Vector((-1 if side=='L' else 1,0,0))
    bend=(pole-direction*pole.dot(direction)).normalized()
    along=(a*a-b*b+distance*distance)/(2*distance)
    goal=shoulder+direction*along+bend*math.sqrt(max(0,a*a-along*along))
    delta=(elbow-shoulder).rotation_difference(goal-shoulder)
    world_rotation(rig,names[0],delta@rig.pose.bones[names[0]].matrix.to_quaternion())
    elbow=rig.pose.bones[names[1]].head.copy();wrist=rig.pose.bones[names[2]].head.copy()
    delta=(wrist-elbow).rotation_difference(target-elbow)
    world_rotation(rig,names[1],delta@rig.pose.bones[names[1]].matrix.to_quaternion())

def main():
    BEFORE.mkdir(parents=True,exist_ok=True);OUT.mkdir(parents=True,exist_ok=True)
    if '--publish' in sys.argv:
        verification=json.loads((WORK/'pose-verification.json').read_text())
        pose=json.loads((WORK/'pose.json').read_text())
        assert verification['passed'] and verification['fbx_roundtrip']
        for name,expected in verification['candidate_sha256'].items():
            assert hashlib.sha256((OUT/name).read_bytes()).hexdigest()==expected,name
        assert hashlib.sha256((ASSETS/'id.blend').read_bytes()).hexdigest() in [pose['source_sha256'],verification['candidate_sha256']['id.blend']], 'Source changed since pose bake'
        for name in ['id.blend','id.fbx','id.morphs.json']:
            shutil.copy2(OUT/name,ASSETS/name)
        records=json.loads((ASSETS/'manifest.json').read_text(encoding='utf-8'))
        row=next(r for r in records if r['slug']=='id')
        for key in ['blend','model']:
            row['sha256'][key]=hashlib.sha256((ROOT/row[key]).read_bytes()).hexdigest()
        row['seated_pose']=pose
        (ASSETS/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
        print('ID_SEATED_POSE_PUBLISHED',flush=True)
        return
    for name in ['id.blend','id.fbx','id.morphs.json','id-rig.json','manifest.json']:
        if not (BEFORE/name).exists():shutil.copy2(ASSETS/name,BEFORE/name)
    source=Path(sys.argv[sys.argv.index('--source')+1]) if '--source' in sys.argv else BEFORE/'id.blend'
    source_digest=hashlib.sha256(source.read_bytes()).hexdigest()
    bpy.ops.wm.open_mainfile(filepath=str(source))
    scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig']
    meshes=[o for o in scene.objects if o.type=='MESH']
    resolve_character_textures(meshes)
    images={n.image for obj in meshes for mat in obj.data.materials if mat and mat.use_nodes
            for n in mat.node_tree.nodes if n.type=='TEX_IMAGE' and n.image}
    for image in images:
        source=Path(bpy.path.abspath(image.filepath))
        assert source.is_file(),source
        dest=OUT/source.name
        if not dest.exists():shutil.copy2(source,dest)
        image.filepath='//'+source.name
    clips=json.loads(scene['clips'])
    names=['Hips','Torso','Head']+[f'{part}.{side}' for side in ['L','R'] for part in ['Thigh','Shin','Foot','UpperArm','Forearm','Hand']]
    frames=[]
    for frame in range(529):
        scene.frame_set(frame);bpy.context.view_layer.update()
        frames.append(dict(matrices={n:rig.pose.bones[n].matrix.copy() for n in ['Root','Mount','Hips']},
                           rotations={n:rig.pose.bones[n].rotation_quaternion.copy() for n in names},
                           locations={n:rig.pose.bones[n].location.copy() for n in names}))
    rest={b.name:b.matrix_local.copy() for b in rig.data.bones}
    wrist_rest={s:rig.data.bones[f'Hand.{s}'].matrix_local.to_quaternion() for s in ['L','R']}
    for frame,saved in enumerate(frames):
        scene.frame_set(frame)
        for name in names:
            p=rig.pose.bones[name];p.rotation_mode='QUATERNION'
            p.rotation_quaternion=saved['rotations'][name];p.location=saved['locations'][name]
        bpy.context.view_layer.update()
        clip,start,end=next(((n,a,b) for n,(a,b) in clips.items() if a<=frame<=b),('Idle',0,60))
        t=(frame-start)/max(1,end-start)
        weight=1.
        if clip=='Death':
            fade=min(1,t/.55);weight=1-fade*fade*(3-2*fade)
        root_delta=saved['matrices']['Root']@rest['Root'].inverted()
        mount_delta=saved['matrices']['Mount']@rest['Mount'].inverted()
        hip_space=root_delta.inverted()@saved['matrices']['Hips']
        hip_rotation=rest['Hips'].to_quaternion().slerp(hip_space.to_quaternion(),.18)
        seated=hip_rotation.to_matrix().to_4x4()
        # Keep breathing/recoil, but do not reuse the on-foot jump's large
        # pelvic lift: world movement and the Mount bone carry the rider.
        sway=(hip_space.translation-rest['Hips'].translation)*.12
        if sway.length>.015:sway.normalize();sway*=.015
        seated.translation=rest['Hips'].translation+HIP_OFFSET+sway
        desired=mount_delta@seated
        old=saved['matrices']['Hips']
        rotation=old.to_quaternion().slerp(desired.to_quaternion(),weight)
        target=rotation.to_matrix().to_4x4();target.translation=old.translation.lerp(desired.translation,weight)
        rig.pose.bones['Hips'].matrix=target
        for part,angle in LEG_ANGLES.items():
            for side in ['L','R']:
                name=f'{part}.{side}'
                rig.pose.bones[name].rotation_quaternion=saved['rotations'][name].slerp(Quaternion((1,0,0),angle),weight)
        for name,angle in [('Torso',-.06),('Head',.04)]:
            rig.pose.bones[name].rotation_quaternion=Quaternion((1,0,0),angle*weight)@saved['rotations'][name]
        bpy.context.view_layer.update()
        if clip in REST_ARMS:
            for side in ['L','R']:
                target=mount_delta@Vector((-.12 if side=='L' else .12,.17,.79))
                arm_ik(rig,side,target)
                # Fingers point forward across the lap, with palms facing down.
                hand=Quaternion((0,1,0),math.pi)@Quaternion((1,0,0),-1.40)@wrist_rest[side]
                world_rotation(rig,f'Hand.{side}',mount_delta.to_quaternion()@hand)
        for name in names:
            p=rig.pose.bones[name]
            p.keyframe_insert(data_path='rotation_quaternion',frame=frame,group=name)
            if name=='Hips':p.keyframe_insert(data_path='location',frame=frame,group=name)
    scene.frame_set(0);bpy.context.view_layer.update()
    report=dict(source=str(source.relative_to(ROOT)),source_sha256=source_digest,hip_offset_m=list(HIP_OFFSET),leg_angles_rad=LEG_ANGLES,clips=clips,rest_arm_clips=sorted(REST_ARMS),
                idle_joints={n:list(rig.pose.bones[n].head) for n in ['Hips','Thigh.L','Shin.L','Foot.L','Hand.L','Hand.R']})
    scene['id_seated_pose']=json.dumps(report)
    bpy.context.preferences.filepaths.save_version=0;bpy.data.use_autopack=False
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'id.blend'),relative_remap=False)
    export_fbx(bpy.data.objects['id_SkinnedMesh'],rig,OUT/'id.fbx')
    report['candidate_sha256']={n:hashlib.sha256((OUT/n).read_bytes()).hexdigest() for n in ['id.blend','id.fbx']}
    (WORK/'pose.json').write_text(json.dumps(report,indent=2))
    review.WORK=WORK
    if '--skip-render' not in sys.argv:review.render_views(scene)
    print('ID_SEATED_POSE_BUILT',flush=True)

if __name__=='__main__':main()
