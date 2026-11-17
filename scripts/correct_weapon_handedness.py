"""Rebake only Attack/Skill arm and torso rotations in the shipped Blender/FBX assets.

Run with Blender --background --python scripts/correct_weapon_handedness.py.
Geometry, skinning, materials, other motion keys and Aya's built-in parasol stay intact.
"""
import hashlib
import json
import math
import shutil
import sys
from pathlib import Path

import bpy
from mathutils import Euler, Matrix

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from build_characters import CLIPS, SLUGS, combat_rotations
from export_fyrox_fbx import export_fbx

OUT = ROOT / 'artifacts/right-hand-fix'
BACKUP = OUT / 'before/assets/characters'
BACKUP.mkdir(parents=True, exist_ok=True)


def mesh_digest(mesh):
    digest = hashlib.sha256()
    for vertex in mesh.data.vertices:
        digest.update(repr((tuple(vertex.co), [(g.group, g.weight) for g in vertex.groups])).encode())
    for polygon in mesh.data.polygons:
        digest.update(repr((tuple(polygon.vertices), polygon.material_index)).encode())
    return digest.hexdigest()


def curves(rig):
    action = rig.animation_data.action
    return action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves


reports = []
for index, slug in enumerate(SLUGS):
    blend = ROOT / f'assets/characters/{slug}.blend'
    fbx = blend.with_suffix('.fbx')
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    scene = bpy.context.scene
    rig = bpy.data.objects['TSB_Rig']
    mesh = bpy.data.objects[f'{slug}_SkinnedMesh']
    # Independent anatomical evidence: bind-pose position and original Tripo finger names.
    assert rig.data.bones['Hand.L'].head_local.x < 0 < rig.data.bones['Hand.R'].head_local.x
    fingers = rig.data.bones['Hand.L'].children_recursive
    assert all(b.name.startswith('mixamorig:RightHand') for b in fingers)
    if index == 4:
        reports.append({'slug': slug, 'right_socket': 'Hand.L', 'unchanged': 'built-in right-hand parasol'})
        continue
    for path in (blend, fbx):
        if not (BACKUP / path.name).exists():
            shutil.copy2(path, BACKUP / path.name)
    before_mesh = mesh_digest(mesh)
    # Compare every key outside the edited channels/frames, including all other clips.
    def untouched_keys():
        edited = set(combat_rotations(index, 'Attack', 0))
        result = []
        for curve in curves(rig):
            changed_channel = any(curve.data_path == f'pose.bones["{bone}"].rotation_{"quaternion" if len(rig.data.bones)>20 else "euler"}' for bone in edited)
            result.append((curve.data_path, curve.array_index, [tuple(p.co) for p in curve.keyframe_points
                if not (changed_channel and (156 <= p.co.x <= 180 or 183 <= p.co.x <= 219))]))
        return result
    before_keys = untouched_keys()
    for motion in ('Attack', 'Skill'):
        start, end = CLIPS[motion]
        for frame in range(start, end + 1):
            scene.frame_set(frame)
            strike = math.sin(math.pi * (frame-start)/(end-start))**2
            for name, angles in combat_rotations(index, motion, strike).items():
                bone = rig.pose.bones[name]
                if len(rig.data.bones) > 20:
                    rest = bone.bone.matrix_local.to_3x3()
                    converter = rest.inverted() @ Matrix.Rotation(math.pi/2, 3, 'X')
                    delta = converter @ Euler(angles).to_matrix() @ converter.inverted()
                    if index != 11 and name.startswith('UpperArm.'):
                        sign = 1 if name.endswith('.R') else -1
                        delta = (rest.inverted() @ Matrix.Rotation(sign*.93, 3, 'Y') @ rest) @ delta
                    bone.rotation_quaternion = delta.to_quaternion()
                    bone.keyframe_insert(data_path='rotation_quaternion', frame=frame, group=name)
                else:
                    bone.rotation_euler = angles
                    bone.keyframe_insert(data_path='rotation_euler', frame=frame, group=name)
    assert untouched_keys() == before_keys, (slug, 'unrelated animation changed')
    assert mesh_digest(mesh) == before_mesh, (slug, 'geometry or weights changed')
    # Check that the attack bends the anatomical right elbow and leaves the support elbow at rest.
    scene.frame_set(168)
    def angle(name):
        bone = rig.pose.bones[name]
        return bone.rotation_quaternion.angle if bone.rotation_mode == 'QUATERNION' else bone.rotation_euler.to_quaternion().angle
    assert angle('Forearm.L') > .5 and angle('Forearm.R') < .001, slug
    scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(blend))
    export_fbx(mesh, rig, fbx)
    reports.append({'slug': slug, 'right_socket': 'Hand.L', 'mesh_sha256': before_mesh,
                    'untouched_keys_preserved': True, 'corrected_clips': ['Attack', 'Skill']})
    print('RIGHT_HAND_ASSET_OK', slug, flush=True)
(OUT / 'asset-verification.json').write_text(json.dumps(reports, indent=2), encoding='utf-8')
print('RIGHT_HAND_ASSETS_COMPLETE', len(reports), flush=True)
