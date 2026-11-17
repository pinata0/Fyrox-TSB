"""Check straight elbows, surface contact, preserved content and FBX poses."""
import hashlib
import json
import sys
from pathlib import Path

import bpy
import numpy as np
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from straighten_chloe_carry import WORK, BEFORE, OUT, FILES, JOINTS, digest, targets


def geometry_hash(obj):
    mesh = obj.data
    h = hashlib.sha256()
    h.update(np.array([v.co[:] for v in mesh.vertices], dtype=np.float32).tobytes())
    h.update(repr([[(g.group, g.weight) for g in v.groups] for v in mesh.vertices]).encode())
    h.update(repr([(tuple(p.vertices), p.material_index) for p in mesh.polygons]).encode())
    for layer in mesh.uv_layers:
        h.update(np.array([v.uv[:] for v in layer.data], dtype=np.float32).tobytes())
    if mesh.shape_keys:
        for key in mesh.shape_keys.key_blocks:
            h.update(key.name.encode())
            h.update(np.array([v.co[:] for v in key.data], dtype=np.float32).tobytes())
    return h.hexdigest()


def preserved(rig):
    action = rig.animation_data.action
    curves = action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
    keys = [(c.data_path, c.array_index,
             [(tuple(k.co), k.interpolation, tuple(k.handle_left), tuple(k.handle_right))
              for k in c.keyframe_points]) for c in curves
            if not any(c.data_path.startswith(f'pose.bones["{n}"]') for n in JOINTS)]
    meshes = {o.name: geometry_hash(o) for o in bpy.context.scene.objects if o.type == 'MESH'
              and not o.name.startswith(('chloe_RefinedSleeve.', 'chloe_RefinedViewSleeve.', 'chloe_RefinedHand.'))}
    return dict(meshes=meshes, animation=keys, bones=[b.name for b in rig.data.bones],
                clips=bpy.context.scene['clips'], spawn=bpy.context.scene.get('spawn_source'))


def main():
    bpy.ops.wm.open_mainfile(filepath=str(BEFORE / 'chloe.blend'))
    before = preserved(bpy.data.objects['TSB_Rig'])
    bpy.ops.wm.open_mainfile(filepath=str(OUT / 'chloe.blend'))
    scene = bpy.context.scene
    rig = bpy.data.objects['TSB_Rig']
    bear = bpy.data.objects['Sebastian']
    assert preserved(rig) == before, 'Unrelated geometry, morphs, UVs, weights or animation changed'
    assert json.loads((BEFORE / 'chloe.morphs.json').read_text()) == json.loads((OUT / 'chloe.morphs.json').read_text())
    report = json.loads((WORK / 'pose.json').read_text())
    profiles = json.loads((OUT / 'chloe-hands.json').read_text())
    errors, deviations, hand_scales, poses = [], [], [], []
    fractional_errors, fractional_deviations, fractional_samples = [], [], []
    first, last = report['frame_range']
    sockets = JOINTS + ['PalmGrip.L', 'PalmGrip.R']
    steps = report['samples_per_frame'] * 2
    for tick in range(first * steps, last * steps + 1):
        frame = tick / steps
        scene.frame_set(int(frame), subframe=frame % 1)
        bpy.context.view_layer.update()
        goals = targets(rig, bear, report['cheek_anchors'], profiles)
        for profile in profiles:
            side = profile['side']
            s, e, w = [rig.pose.bones[f'{p}.{side}'].head.copy()
                       for p in ('UpperArm', 'Forearm', 'Hand')]
            line = (w - s).normalized()
            deviation = ((e - s) - line * (e - s).dot(line)).length
            hand = rig.pose.bones[f'Hand.{side}']
            delta = hand.matrix @ hand.bone.matrix_local.inverted()
            error = (delta @ Vector(profile['contact']) - goals[side][2]).length
            if tick % 2:
                fractional_errors.append(error)
                fractional_deviations.append(deviation)
                fractional_samples.append((error, deviation, frame, side))
            else:
                errors.append(error)
                deviations.append(deviation)
                hand_scales.extend(abs(x - 1.) for x in hand.matrix.to_scale())
        if tick % steps == 0:
            poses.append([list(rig.matrix_world @ rig.pose.bones[n].head) for n in sockets])
    assert max(errors) < 1e-5, max(errors)
    assert max(deviations) < 1e-5, max(deviations)
    assert max(hand_scales) < 1e-5, max(hand_scales)
    print('INTERPOLATION_PEAKS', sorted(fractional_samples, reverse=True)[:10], flush=True)
    # The editable scene retains the original nonlinear body curves. Runtime
    # FBX samples all bones together; check its interpolation separately below.
    assert max(fractional_errors) < .01, max(fractional_errors)
    assert max(fractional_deviations) < .003, max(fractional_deviations)
    anchor_points = {side: [list(bear.matrix_world @ bear.data.vertices[i].co)
                           for i in anchor['vertices']]
                     for side, anchor in report['cheek_anchors'].items()}
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = 30
    bpy.ops.import_scene.fbx(filepath=str(OUT / 'chloe.fbx'))
    rig = next(o for o in bpy.data.objects if o.type == 'ARMATURE')
    assert len(rig.data.bones) == len(before['bones'])
    assert len(bpy.data.objects['chloe_SkinnedMesh'].data.shape_keys.key_blocks) == 24
    start, end = rig.animation_data.action.frame_range
    assert abs(end - start - (last - first)) < 1e-4
    imported = []
    for frame in range(last - first + 1):
        bpy.context.scene.frame_set(round(start) + frame)
        bpy.context.view_layer.update()
        imported.append([list(rig.matrix_world @ rig.pose.bones[n].head) for n in sockets])
    roundtrip_error = float(np.max(np.linalg.norm(np.array(poses) - np.array(imported), axis=2)))
    assert roundtrip_error < .0001, roundtrip_error
    from mathutils.kdtree import KDTree
    bear = bpy.data.objects['Sebastian']
    tree = KDTree(len(bear.data.vertices))
    for v in bear.data.vertices:
        tree.insert(bear.matrix_world @ v.co, v.index)
    tree.balance()
    anchors = {}
    for side, points in anchor_points.items():
        matches = [tree.find(Vector(point)) for point in points]
        assert max(row[2] for row in matches) < 1e-5
        anchors[side] = [row[1] for row in matches]
    runtime_errors, runtime_elbows = [], []
    for tick in range((last - first) * steps + 1):
        frame = start + tick / steps
        bpy.context.scene.frame_set(int(frame), subframe=frame % 1)
        bpy.context.view_layer.update()
        evaluated = bear.evaluated_get(bpy.context.evaluated_depsgraph_get())
        head = rig.matrix_world @ rig.pose.bones['Sebastian.Head'].matrix @ rig.data.bones['Sebastian.Head'].matrix_local.inverted() @ rig.matrix_world.inverted()
        for profile in profiles:
            side = profile['side']
            cheek = sum((bear.matrix_world @ evaluated.data.vertices[i].co * w
                         for i, w in zip(anchors[side], report['cheek_anchors'][side]['weights'])), Vector())
            hand = rig.pose.bones[f'Hand.{side}']
            delta = rig.matrix_world @ hand.matrix @ hand.bone.matrix_local.inverted() @ rig.matrix_world.inverted()
            runtime_errors.append((delta @ Vector(profile['contact']) - cheek - head.to_3x3() @ Vector((0, 0, -.004))).length)
            s, e, w = [rig.matrix_world @ rig.pose.bones[f'{p}.{side}'].head
                       for p in ('UpperArm', 'Forearm', 'Hand')]
            line = (w - s).normalized()
            runtime_elbows.append(((e - s) - line * (e - s).dot(line)).length)
    print('FBX_RUNTIME_CONTACT', max(runtime_errors), 'ELBOW', max(runtime_elbows), flush=True)
    assert max(runtime_errors) < .003, max(runtime_errors)
    assert max(runtime_elbows) < .003, max(runtime_elbows)
    verification = dict(passed=True, frames=last - first + 1, baked_samples=len(errors)//2,
                        interpolated_samples=len(fractional_errors)//2,
                        preserved_body_companion_morphs_uv_weights=True, preserved_other_animation=True,
                        preserved_spawn_body_animation=bool(before['spawn']),
                        maximum_palm_contact_error_m=max(errors), maximum_elbow_line_error_m=max(deviations),
                        maximum_interpolated_contact_error_m=max(fractional_errors),
                        maximum_interpolated_elbow_line_error_m=max(fractional_deviations),
                        maximum_hand_scale_error=max(hand_scales), fbx_roundtrip_error_m=roundtrip_error,
                        maximum_fbx_interpolated_contact_error_m=max(runtime_errors),
                        maximum_fbx_interpolated_elbow_line_error_m=max(runtime_elbows),
                        sha256={name: digest(OUT / name) for name in FILES})
    (WORK / 'verification.json').write_text(json.dumps(verification, indent=2))
    print('CHLOE_STRAIGHT_ARMS_VERIFY_OK', verification, flush=True)


if __name__ == '__main__':
    main()
