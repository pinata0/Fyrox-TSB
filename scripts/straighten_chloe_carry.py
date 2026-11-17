"""Straighten Chloe's raised arms while keeping her palms under Sebastian.

Run in Blender with --background --factory-startup --python-exit-code 1 --python.
Build from a once-only backup of the current carry asset, run
verify_chloe_straight_carry.py, then --publish. Existing body/companion
animation and expression geometry are kept.
"""
import hashlib
import json
import shutil
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Quaternion, Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from export_fyrox_fbx import export_fbx, resolve_character_textures
from pose_chloe_carry import palm_rotation, stretch
import resize_companions as review

WORK = ROOT / 'artifacts/chloe-straight-arms'
BEFORE = WORK / 'before'
OUT = WORK / 'candidate'
ASSETS = ROOT / 'assets/characters'
FILES = ['chloe.blend', 'chloe.fbx', 'chloe.morphs.json', 'chloe-hands.json']
JOINTS = [f'{part}.{side}' for side in ('L', 'R')
          for part in ('UpperArm', 'Forearm', 'Hand')]
SAMPLES_PER_FRAME = 4


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def targets(rig, bear, anchors, profiles):
    evaluated = bear.evaluated_get(bpy.context.evaluated_depsgraph_get())
    head = rig.pose.bones['Sebastian.Head'].matrix @ rig.data.bones['Sebastian.Head'].matrix_local.inverted()
    result = {}
    for profile in profiles:
        side = profile['side']
        anchor = anchors[side]
        cheek = sum((evaluated.data.vertices[i].co * w
                     for i, w in zip(anchor['vertices'], anchor['weights'])), Vector())
        contact = cheek + head.to_3x3() @ Vector((0, 0, -.004))
        rotation = head.to_quaternion() @ palm_rotation(profile, side)
        wrist = contact - rotation @ (Vector(profile['contact']) - Vector(profile['wrist']))
        result[side] = (wrist, rotation, contact)
    return result


def set_matrix(rig, name, point, rotation, scale=1.):
    # Uniform scale is deliberate: FBX TRS can represent it without shear,
    # including the inverse scale that keeps palms/fingers their original size.
    rig.pose.bones[name].matrix = Matrix.LocRotScale(point, rotation, Vector((scale,) * 3))
    bpy.context.view_layer.update()


def build():
    BEFORE.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    for name in FILES:
        if not (BEFORE / name).exists():
            shutil.copy2(ASSETS / name, BEFORE / name)
    bpy.ops.wm.open_mainfile(filepath=str(BEFORE / 'chloe.blend'))
    scene = bpy.context.scene
    rig = bpy.data.objects['TSB_Rig']
    bear = bpy.data.objects['Sebastian']
    body = bpy.data.objects['chloe_SkinnedMesh']
    carry = json.loads(scene['chloe_carry_pose'])
    profiles = json.loads((BEFORE / 'chloe-hands.json').read_text())
    meshes = [o for o in scene.objects if o.type == 'MESH']
    resolve_character_textures(meshes)
    for obj in meshes:
        for mat in obj.data.materials:
            if not mat or not mat.use_nodes:
                continue
            for node in mat.node_tree.nodes:
                if node.type == 'TEX_IMAGE' and node.image:
                    path = Path(bpy.path.abspath(node.image.filepath))
                    if path.resolve() != (OUT / path.name).resolve():
                        shutil.copy2(path, OUT / path.name)
                    node.image.filepath = str(OUT / path.name)
    scene.frame_set(0)
    bpy.context.view_layer.update()
    neutral = targets(rig, bear, carry['cheek_anchors'], profiles)
    # Fit both sleeves to the neutral reach, preserving their width and hands.
    reach = sum((neutral[s][0] - rig.pose.bones[f'UpperArm.{s}'].head).length
                for s in ('L', 'R')) / 2
    offsets = {}
    lengths = {}
    for profile in profiles:
        side = profile['side']
        s, e, w = [rig.data.bones[f'{p}.{side}'].head_local.copy()
                   for p in ('UpperArm', 'Forearm', 'Hand')]
        a, b = (e - s).length, (w - e).length
        da, db = (e - s).normalized(), (w - e).normalized()
        factor = reach / (a + b)
        new_e = s + (e - s) * factor
        new_w = new_e + (w - e) * factor
        offsets[side] = (new_e - e, new_w - w)
        lengths[side] = (a * factor, b * factor)
        for obj in meshes:
            if obj.name in (f'chloe_RefinedSleeve.{side}', f'chloe_RefinedViewSleeve.{side}'):
                for v in obj.data.vertices:
                    weights = {obj.vertex_groups[g.group].name: g.weight for g in v.groups}
                    u, f = weights.get(f'UpperArm.{side}', 0.), weights.get(f'Forearm.{side}', 0.)
                    assert u + f > .999
                    v.co = (stretch(v.co, s, da, factor, Vector()) * u
                            + stretch(v.co, e, db, factor, new_e - e) * f) / (u + f)
                obj.data.update()
            elif obj.name == f'chloe_RefinedHand.{side}':
                for v in obj.data.vertices:
                    v.co += new_w - w
                obj.data.update()
        profile['wrist'] = list(new_w)
        profile['contact'] = list(Vector(profile['contact']) + new_w - w)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode='EDIT')
    for bone in rig.data.edit_bones:
        for side, (elbow, wrist) in offsets.items():
            if bone.name == f'Forearm.{side}':
                delta = elbow
            elif bone.name.endswith('.' + side) and bone.name.startswith(('Hand.', 'PalmGrip.', 'Index', 'Middle', 'Ring', 'Pinky', 'Thumb')):
                delta = wrist
            else:
                continue
            bone.head += delta
            bone.tail += delta
    bpy.ops.object.mode_set(mode='OBJECT')
    rest = {b.name: b.matrix_local.copy() for b in rig.data.bones}
    scales = []
    previous = {}
    # Match nonlinear body/companion motion between the original 30 Hz keys.
    for tick in range(scene.frame_start * SAMPLES_PER_FRAME, scene.frame_end * SAMPLES_PER_FRAME + 1):
        frame = tick / SAMPLES_PER_FRAME
        scene.frame_set(int(frame), subframe=frame % 1)
        for name in JOINTS:
            p = rig.pose.bones[name]
            p.rotation_mode = 'QUATERNION'
            p.rotation_quaternion = Quaternion()
            p.location = Vector()
            p.scale = Vector((1, 1, 1))
        bpy.context.view_layer.update()
        goals = targets(rig, bear, carry['cheek_anchors'], profiles)
        torso = (rig.pose.bones['Torso'].matrix @ rest['Torso'].inverted()).to_quaternion()
        for side in ('L', 'R'):
            upper, fore, hand = [f'{p}.{side}' for p in ('UpperArm', 'Forearm', 'Hand')]
            shoulder = rig.pose.bones[upper].head.copy()
            wrist, palm, _ = goals[side]
            direction = (wrist - shoulder).normalized()
            a, b = lengths[side]
            scale = (wrist - shoulder).length / (a + b)
            scales.append(scale)
            elbow = shoulder + (wrist - shoulder) * (a / (a + b))
            for name, child, origin in ((upper, fore, shoulder), (fore, hand, elbow)):
                axis = rest[child].translation - rest[name].translation
                rotation = (torso @ axis).rotation_difference(direction) @ torso
                set_matrix(rig, name, origin, rotation @ rest[name].to_quaternion(), scale)
            set_matrix(rig, hand, wrist, palm @ rest[hand].to_quaternion())
        for name in JOINTS:
            p = rig.pose.bones[name]
            if name in previous and p.rotation_quaternion.dot(previous[name]) < 0:
                p.rotation_quaternion.negate()
            previous[name] = p.rotation_quaternion.copy()
            for path in ('rotation_quaternion', 'location', 'scale'):
                p.keyframe_insert(data_path=path, frame=frame, group=name)
    action = rig.animation_data.action
    curves = action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
    for curve in curves:
        if any(curve.data_path.startswith(f'pose.bones["{n}"]') for n in JOINTS):
            for key in curve.keyframe_points:
                key.interpolation = 'LINEAR'
    scene.frame_set(0)
    bpy.context.view_layer.update()
    report = dict(source_sha256={n: digest(BEFORE / n) for n in FILES},
                  clips=json.loads(scene['clips']), frame_range=[scene.frame_start, scene.frame_end],
                  neutral_reach_m=reach, lengths_m=lengths, arm_scale_range=[min(scales), max(scales)],
                  samples_per_frame=SAMPLES_PER_FRAME,
                  cheek_anchors=carry['cheek_anchors'], elbow_bend_degrees=0.)
    scene['chloe_straight_carry_pose'] = json.dumps(report)
    scene['hand_rig'] = json.dumps(profiles)
    (OUT / 'chloe-hands.json').write_text(json.dumps(profiles, indent=2))
    for image in bpy.data.images:
        if image.source == 'FILE' and image.filepath and Path(bpy.path.abspath(image.filepath)).is_file():
            image.filepath = '//' + Path(bpy.path.abspath(image.filepath)).name
    bpy.context.preferences.filepaths.save_version = 0
    bpy.data.use_autopack = False
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT / 'chloe.blend'), relative_remap=False)
    export_fbx(body, rig, OUT / 'chloe.fbx', bake_step=1 / SAMPLES_PER_FRAME)
    (WORK / 'pose.json').write_text(json.dumps(report, indent=2))
    if '--skip-render' not in sys.argv:
        review.WORK = WORK
        review.render(scene, 'chloe', 'straight', ('front', 'side', 'three-quarter'))
    print('CHLOE_STRAIGHT_ARMS_BUILT', report, flush=True)


def publish():
    verification = json.loads((WORK / 'verification.json').read_text())
    report = json.loads((WORK / 'pose.json').read_text())
    assert verification['passed']
    for name, expected in verification['sha256'].items():
        assert digest(OUT / name) == expected, name
        assert digest(ASSETS / name) in (report['source_sha256'][name], expected), f'{name} changed independently'
    for name in FILES:
        shutil.copy2(OUT / name, ASSETS / name)
    records = json.loads((ASSETS / 'manifest.json').read_text(encoding='utf-8'))
    row = next(r for r in records if r['slug'] == 'chloe')
    row['straight_carry_pose'] = report
    for key in ('blend', 'model'):
        row['sha256'][key] = digest(ROOT / row[key])
    (ASSETS / 'manifest.json').write_text(json.dumps(records, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print('CHLOE_STRAIGHT_ARMS_PUBLISHED', flush=True)


if __name__ == '__main__':
    publish() if '--publish' in sys.argv else build()
