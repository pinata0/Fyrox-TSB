"""Retarget verified standing dances into only frames 285..405 of existing rigs.

Run the Spine sampler first, then Blender --background --python this_file.
The screen-plane joint directions come from the source. Depth and foot contact
are a 3D adaptation; meshes, weights and every other animation key are preserved.
"""
import hashlib
import json
import math
import shutil
import sys
from pathlib import Path
import bpy
from mathutils import Matrix, Quaternion, Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from export_fyrox_fbx import export_fbx

OUT = ROOT / 'artifacts/dance-research'
BACKUP = OUT / 'before/assets/characters'
BACKUP.mkdir(parents=True, exist_ok=True)
DATA = json.loads((OUT / 'sampled-dances.json').read_text(encoding='utf-8'))

def curves(rig):
    return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves

def mesh_hash(mesh):
    h = hashlib.sha256()
    for v in mesh.data.vertices:
        h.update(repr((tuple(v.co), [(g.group, g.weight) for g in v.groups])).encode())
    for p in mesh.data.polygons:
        h.update(repr((tuple(p.vertices), p.material_index)).encode())
    return h.hexdigest()

def set_rotation(p, q):
    if p.rotation_mode == 'QUATERNION':
        # Maintain a consistent hemisphere across frames for baked interpolation.
        if p.rotation_quaternion.dot(q) < 0: q = -q
        p.rotation_quaternion = q
    else:
        p.rotation_euler = q.to_euler('XYZ', p.rotation_euler)

def world_rotation(p, delta):
    rest = p.bone.matrix_local.to_3x3()
    parent_space = (p.parent.matrix.to_3x3() @ p.parent.bone.matrix_local.to_3x3().inverted() @ rest) if p.parent else rest
    set_rotation(p, (parent_space.inverted() @ delta.to_matrix() @ rest).to_quaternion())
    bpy.context.view_layer.update()

def aim(rig, name, child, a, b, depth):
    p = rig.pose.bones[name]
    bind = rig.data.bones[child].head_local - p.bone.head_local
    v = Vector((b[0]-a[0], 0, b[1]-a[1]))
    v.y = depth * v.length
    world_rotation(p, bind.normalized().rotation_difference(v.normalized()))

reports = []
only = sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
for record in DATA:
    slug = record['slug']
    if only and slug not in only: continue
    blend = ROOT / f'assets/characters/{slug}.blend'
    fbx = blend.with_suffix('.fbx')
    for path in (blend, fbx):
        if not (BACKUP / path.name).exists(): shutil.copy2(path, BACKUP / path.name)
    # Read the current file so unrelated edits made during this task are retained.
    # Every dance frame is rebuilt from Idle, making repeated bakes idempotent.
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    rig = bpy.data.objects['TSB_Rig']
    mesh = bpy.data.objects[f'{slug}_SkinnedMesh']
    scene = bpy.context.scene
    def other_keys():
        return [(c.data_path,c.array_index,[tuple(p.co) for p in c.keyframe_points if not 285<=p.co.x<=405]) for c in curves(rig)]
    before_keys = other_keys()
    geometry = mesh_hash(mesh)
    scene.frame_set(0)
    base = {p.name: (p.rotation_quaternion.copy() if p.rotation_mode=='QUATERNION' else p.rotation_euler.to_quaternion(), p.location.copy(),p.scale.copy()) for p in rig.pose.bones}
    source = record['samples']
    center_x = sum(s['pelvis'][0] for s in source[:-1]) / 120
    center_z = sum(s['pelvis'][1] for s in source[:-1]) / 120
    source_leg = sum(math.dist(s['legs']['R'][0],s['legs']['R'][1])+math.dist(s['legs']['R'][1],s['legs']['R'][2]) for s in source[:-1])/120
    target_leg = (rig.data.bones['Thigh.L'].head_local-rig.data.bones['Shin.L'].head_local).length+(rig.data.bones['Shin.L'].head_local-rig.data.bones['Foot.L'].head_local).length
    scale = target_leg / source_leg
    floors = []
    first_pose = None
    for i, sample in enumerate(source):
        frame = 285+i
        scene.frame_set(frame)
        for p in rig.pose.bones:
            q, loc, scl = base[p.name]
            set_rotation(p, q); p.location=loc; p.scale=scl
        hips = rig.pose.bones['Hips']
        hips.location = base['Hips'][1] + hips.bone.matrix_local.to_3x3().inverted() @ Vector(((sample['pelvis'][0]-center_x)*scale,0,(sample['pelvis'][1]-center_z)*scale))
        bpy.context.view_layer.update()
        tilt = math.atan2(sample['head'][0]-sample['pelvis'][0],sample['head'][1]-sample['pelvis'][1])
        world_rotation(hips,Quaternion((0,1,0),tilt*.2))
        world_rotation(rig.pose.bones['Torso'],Quaternion((0,1,0),tilt))
        world_rotation(rig.pose.bones['Head'],Quaternion((0,1,0),-math.radians(sample['headAngle'])))
        for target_side, source_side in [('L','R'),('R','L')]:
            shoulder, elbow, wrist = sample['arms'][source_side]
            # Inferred forward depth keeps raised hands in front of the large 3D head.
            if not (slug=='aya' and target_side=='L'):
                aim(rig,'UpperArm.'+target_side,'Forearm.'+target_side,shoulder,elbow,-.24)
                aim(rig,'Forearm.'+target_side,'Hand.'+target_side,elbow,wrist,-.34)
            thigh,knee,ankle = sample['legs'][source_side]
            aim(rig,'Thigh.'+target_side,'Shin.'+target_side,thigh,knee,-.08)
            aim(rig,'Shin.'+target_side,'Foot.'+target_side,knee,ankle,.08)
            world_rotation(rig.pose.bones['Foot.'+target_side],Quaternion())
        # Keep the right hand's built-in parasol grip coherent with the source arm.
        # Secondary motion follows the sampled torso tilt, not the previous generic beat.
        for name, factor in [('Hair',-.22),('Cape',-.15),('Tail',-.3)]:
            if name in rig.pose.bones:
                p=rig.pose.bones[name]
                set_rotation(p,base[name][0] @ Quaternion((0,0,1),tilt*factor))
        bpy.context.view_layer.update()
        evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        floor=min((evaluated.matrix_world@v.co).z for v in evaluated.data.vertices)
        # Preserve float height for Renua. Grounded heroes keep the supporting foot planted.
        desired_floor = .10 if slug=='renua' else .012
        hips.location += hips.bone.matrix_local.to_3x3().inverted() @ Vector((0,0,desired_floor-floor))
        floors.append(round(floor,5))
        if i==0:
            first_pose={p.name:(p.rotation_quaternion.copy() if p.rotation_mode=='QUATERNION' else p.rotation_euler.copy(),p.location.copy()) for p in rig.pose.bones}
        if i==120:
            for p in rig.pose.bones:
                q,loc=first_pose[p.name]
                if p.rotation_mode=='QUATERNION':p.rotation_quaternion=q
                else:p.rotation_euler=q
                p.location=loc
        for p in rig.pose.bones:
            p.keyframe_insert(data_path='rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=p.name)
            p.keyframe_insert(data_path='location',frame=frame,group=p.name)
    assert other_keys()==before_keys,(slug,'unrelated animation keys changed')
    assert mesh_hash(mesh)==geometry,(slug,'geometry or weights changed')
    scene['dance_source']='Standing Normal / Dance_1; sampled screen-plane joints; inferred 3D depth'
    scene['dance_seconds']=record['duration']
    scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(blend))
    export_fbx(mesh,rig,fbx)
    manifest_path=ROOT/'assets/characters/manifest.json'
    manifest=json.loads(manifest_path.read_text(encoding='utf-8'))
    item=next(item for item in manifest if item['slug']==slug)
    for key,path in [('blend',blend),('model',fbx)]:
        item['sha256'][key]=hashlib.sha256(path.read_bytes()).hexdigest()
    item['dance']={'status':'verified-standing-retarget','clip':record['clip'],'seconds':record['duration'],
                   'source':record['source']['viewer'],'source_sha256':record['sha256']}
    manifest_path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    reports.append({'slug':slug,'duration':record['duration'],'clip':record['clip'],'source_sha256':record['sha256'],'mesh_sha256':geometry,'unrelated_keys_preserved':True,'pre_contact_floor':floors})
    print('DANCE_ASSET_OK',slug,record['duration'],flush=True)
(OUT/'asset-verification.json').write_text(json.dumps(reports,indent=2),encoding='utf-8')
print('DANCE_ASSETS_COMPLETE',len(reports),flush=True)
