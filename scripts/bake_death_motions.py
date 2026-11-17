"""Bake only Death in all shipped rigs. Preserve geometry, weights and every unrelated key.
Run: blender --background --python scripts/bake_death_motions.py
"""
import hashlib, json, math, shutil, sys
from pathlib import Path
import bpy
from mathutils import Matrix, Vector
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from death_motion_data import DATA,sample
from export_fyrox_fbx import export_fbx
OUT=ROOT/'artifacts/death-replay'
BACKUP=OUT/'before/assets/characters';BACKUP.mkdir(parents=True,exist_ok=True)

def curves(rig):
    return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves

def mesh_hash(mesh):
    h=hashlib.sha256()
    for v in mesh.data.vertices: h.update(repr((tuple(v.co),[(g.group,g.weight) for g in v.groups])).encode())
    for p in mesh.data.polygons: h.update(repr((tuple(p.vertices),p.material_index)).encode())
    return h.hexdigest()

reports=[];checkpoints=[]
for index,record in enumerate(DATA):
    slug=record['slug'];blend=ROOT/f'assets/characters/{slug}.blend';fbx=blend.with_suffix('.fbx')
    for path in (blend,fbx):
        if not (BACKUP/path.name).exists():shutil.copy2(path,BACKUP/path.name)
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects[f'{slug}_SkinnedMesh'];scene=bpy.context.scene
    def other_keys():
        return [(c.data_path,c.array_index,[tuple(p.co) for p in c.keyframe_points if not 423<=p.co.x<=453]) for c in curves(rig)]
    before=other_keys();geometry=mesh_hash(mesh)
    scene.frame_set(0)
    base={p.name:(p.rotation_quaternion.copy() if p.rotation_mode=='QUATERNION' else p.rotation_euler.to_quaternion(),p.location.copy(),p.scale.copy()) for p in rig.pose.bones}
    floors=[]
    for frame in range(423,454):
        scene.frame_set(frame)
        for p in rig.pose.bones:
            rotation,location,scale=base[p.name]
            if p.rotation_mode=='QUATERNION':p.rotation_quaternion=rotation
            else:p.rotation_euler=rotation.to_euler('XYZ')
            p.location=location;p.scale=scale
        rotations,offset=sample(index,(frame-423)/30)
        for name,angle in rotations.items():
            p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3()
            delta=rest.inverted()@Matrix.Rotation(angle,3,'X')@rest
            q=base[name][0]@delta.to_quaternion()
            if p.rotation_mode=='QUATERNION':p.rotation_quaternion=q
            else:p.rotation_euler=q.to_euler('XYZ')
        hips=rig.pose.bones['Hips'];rest=hips.bone.matrix_local.to_3x3()
        hips.location=base['Hips'][1]+rest.inverted()@Vector(offset)
        # Prevent ground penetration caused by mapping flat sprite proportions to a solid 3D body.
        bpy.context.view_layer.update()
        evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        floor=min((evaluated.matrix_world@v.co).z for v in evaluated.data.vertices)
        if floor<.012:
            hips.location+=rest.inverted()@Vector((0,0,.012-floor))
        floors.append(round(floor,5))
        for p in rig.pose.bones:
            p.keyframe_insert(data_path='rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=p.name)
            p.keyframe_insert(data_path='location',frame=frame,group=p.name)
        bpy.context.view_layer.update()
    assert other_keys()==before,(slug,'unrelated keys changed')
    assert mesh_hash(mesh)==geometry,(slug,'geometry changed')
    poses=[]
    for phase in [0,.32,.65,1]:
        frame=423+phase*30;scene.frame_set(int(frame),subframe=frame-int(frame));bpy.context.view_layer.update()
        poses.append({name:list(rig.matrix_world@rig.pose.bones[name].head) for name in ['Hips','Head']})
    checkpoints.append({'slug':slug,'poses':poses})
    scene['death_source']='Original battle Die: sampled at 30 Hz; retargeted to 3D; normalized frames 423-453'
    scene['death_seconds']=record['duration'];scene.render.fps=30;scene.render.fps_base=1.0;scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(blend));export_fbx(mesh,rig,fbx)
    reports.append({'slug':slug,'duration':record['duration'],'mesh_sha256':geometry,'unrelated_keys_preserved':True,'pre_correction_floors':floors})
    (OUT/'asset-verification.json').write_text(json.dumps(reports,indent=2),encoding='utf-8')
    print('DEATH_ASSET_OK',slug,record['duration'],flush=True)
(ROOT/'assets/characters/death/pose-checkpoints.json').write_text(json.dumps(checkpoints),encoding='utf-8')
print('DEATH_ASSETS_COMPLETE',len(reports),flush=True)
