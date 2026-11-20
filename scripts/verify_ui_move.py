"""Inspect the saved Ui hop and FBX; verify unrelated motion/geometry preservation.

Blender --background --python-exit-code 1 --python scripts/verify_ui_move.py
"""
import hashlib
import json
import zipfile
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/move-motions'
CLIPS = {'Walk': (63, 93), 'Run': (96, 120)}


def digest(value):
    return hashlib.sha256(repr(value).encode()).hexdigest()


def preserved():
    rig = bpy.data.objects['TSB_Rig']
    action = rig.animation_data.action
    curves = action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
    keys = sorted((c.data_path, c.array_index, [
        (tuple(p.co), p.interpolation) for p in c.keyframe_points
        if not any(start <= p.co.x <= end for start, end in CLIPS.values())
    ]) for c in curves)
    meshes = {}
    for obj in bpy.context.scene.objects:
        if obj.type != 'MESH':
            continue
        m = obj.data
        meshes[obj.name] = digest((
            [(tuple(v.co), [(g.group, g.weight) for g in v.groups]) for v in m.vertices],
            [(tuple(p.vertices), p.material_index) for p in m.polygons],
            [(uv.name, [tuple(p.uv) for p in uv.data]) for uv in m.uv_layers],
            [(k.name, [tuple(p.co) for p in k.data]) for k in m.shape_keys.key_blocks]
            if m.shape_keys else [],
        ))
    return {'unrelated_keys': digest(keys), 'meshes': meshes}


def inspect(offset):
    scene = bpy.context.scene
    rig = next(o for o in scene.objects if o.type == 'ARMATURE')
    frog = bpy.data.objects['Frog']
    result = {}
    for clip, (start, end) in CLIPS.items():
        heights, roots, hips, legs, scales, poses = [], [], [], [], [], []
        # Quarter-frame samples catch penetration between authored keys.
        for step in range((end - start) * 4 + 1):
            frame = start + offset + step / 4
            scene.frame_set(int(frame), subframe=frame % 1)
            bpy.context.view_layer.update()
            evaluated = frog.evaluated_get(bpy.context.evaluated_depsgraph_get())
            heights.append(min((evaluated.matrix_world @ v.co).z for v in evaluated.data.vertices))
            roots.append(tuple(rig.pose.bones['Root'].matrix.translation))
            hips.append((rig.matrix_world @ rig.pose.bones['Hips'].matrix).translation.z)
            legs.append(rig.pose.bones['Frog.Leg.L'].matrix_basis.to_quaternion())
            scales.append(rig.pose.bones['FrogRoot'].scale.y)
            poses.append([v for p in rig.pose.bones for row in p.matrix for v in row])
        seam = max(abs(a-b) for a, b in zip(poses[0], poses[-1]))
        leg_swing = max(legs[0].rotation_difference(q).angle for q in legs)
        assert min(heights) >= -.002, (clip, 'frog penetrates floor', min(heights))
        assert max(heights) - min(heights) > .38, (clip, 'frog not airborne', heights)
        assert max(hips) - min(hips) > .30, (clip, 'rider does not follow hop')
        assert leg_swing > .10, (clip, 'frog legs frozen', leg_swing)
        assert max(scales) - min(scales) > .05, (clip, 'no landing squash')
        assert max(max(r[i] for r in roots) - min(r[i] for r in roots) for i in range(3)) < .0001
        assert seam < .0001, (clip, 'loop seam', seam)
        result[clip] = {
            'frog_floor_m': [min(heights), max(heights)],
            'rider_height_m': [min(hips), max(hips)],
            'frog_leg_swing_rad': leg_swing,
            'frog_height_scale': [min(scales), max(scales)],
            'loop_seam': seam,
            'root_stationary': True,
        }
    return result


before_dir = OUT / 'verification-before'
before_dir.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(OUT / 'before/assets/characters/ui.zip') as archive:
    (before_dir / 'ui.blend').write_bytes(archive.read('ui.blend'))
bpy.ops.wm.open_mainfile(filepath=str(before_dir / 'ui.blend'))
before = preserved()
bpy.ops.wm.open_mainfile(filepath=str(ROOT / 'assets/characters/ui.blend'))
after = preserved()
assert before == after, 'unrelated animation, mesh, weights, UVs or morphs changed'
report = {'unrelated_keys_and_geometry_preserved': True, 'blend': inspect(0)}

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=str(ROOT / 'assets/characters/ui.fbx'))
# Blender imports the exported frame zero at frame one.
report['fbx'] = inspect(1)
(OUT / 'ui-verification.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
print('UI_MOVE_ASSET_VERIFIED', json.dumps(report), flush=True)
