"""Retarget archived Move into Walk/Run, preserving all other clips and geometry.

Run extract_move_motions.cjs, then Blender --background --python-exit-code 1
--python scripts/bake_move_motions.py [-- slug ...]. Runtime restores source timing.
"""
import hashlib
import json
import math
import sys
import zipfile
from pathlib import Path
import bpy
from mathutils import Matrix, Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from export_fyrox_fbx import export_fbx
import ui_move

OUT = ROOT / 'artifacts/move-motions'
BACKUP = OUT / 'before/assets/characters'
BACKUP.mkdir(parents=True, exist_ok=True)
DATA = json.loads((ROOT / 'assets/characters/move/motions.json').read_text(encoding='utf-8'))
CLIPS = [(63, 93), (96, 120)]


def curves(rig):
    action = rig.animation_data.action
    return action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves


def mesh_hash(mesh):
    h = hashlib.sha256()
    for v in mesh.data.vertices:
        h.update(repr((tuple(v.co), [(g.group, g.weight) for g in v.groups])).encode())
    for p in mesh.data.polygons:
        h.update(repr((tuple(p.vertices), p.material_index)).encode())
    for key in mesh.data.shape_keys.key_blocks if mesh.data.shape_keys else []:
        h.update(key.name.encode())
        for p in key.data:
            h.update(repr(tuple(p.co)).encode())
    return h.hexdigest()


def rotation(p, q):
    if p.rotation_mode == 'QUATERNION':
        if p.rotation_quaternion.dot(q) < 0:
            q = -q
        p.rotation_quaternion = q
    else:
        p.rotation_euler = q.to_euler('XYZ', p.rotation_euler)


def transforms(rig):
    return {p.name: (p.rotation_quaternion.copy() if p.rotation_mode == 'QUATERNION'
                    else p.rotation_euler.to_quaternion(), p.location.copy(), p.scale.copy())
            for p in rig.pose.bones}


def restore(rig, pose):
    for p in rig.pose.bones:
        q, loc, scale = pose[p.name]
        rotation(p, q)
        p.location, p.scale = loc, scale


only = sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
report_path = OUT / 'asset-verification.json'
reports = json.loads(report_path.read_text()) if only and report_path.exists() else []
for record in DATA:
    slug = record['slug']
    if only and slug not in only:
        continue
    blend = ROOT / f'assets/characters/{slug}.blend'
    fbx = blend.with_suffix('.fbx')
    archive = BACKUP / f'{slug}.zip'
    if not archive.exists():
        with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED) as backup:
            for path in (blend, fbx):
                backup.write(path, path.name)
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    rig = bpy.data.objects['TSB_Rig']
    mesh = bpy.data.objects[f'{slug}_SkinnedMesh']
    meshes = [o for o in bpy.context.scene.objects if o.type == 'MESH'
              and any(m.type == 'ARMATURE' and m.object == rig for m in o.modifiers)]
    scene = bpy.context.scene

    def other_keys():
        return [(c.data_path, c.array_index, [(tuple(p.co), p.interpolation) for p in c.keyframe_points
                 if not any(s <= p.co.x <= e for s, e in CLIPS)]) for c in curves(rig)]

    before = other_keys()
    geometry = {o.name: mesh_hash(o) for o in meshes}
    scene.frame_set(0)
    base = transforms(rig)
    bpy.context.view_layer.update()
    evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    base_floor = min((evaluated.matrix_world @ v.co).z for v in evaluated.data.vertices)
    # Keep source bobbing around the model's own ground/float height. Never transfer
    # source translation into actor motion: collision and server movement own that.
    center = [sum(s['root'][j] for s in record['samples'][:-1]) / (len(record['samples'])-1) for j in range(2)]
    checks = []
    for start, end in CLIPS:
        first_pose = None
        for frame in range(start, end+1):
            phase = (frame-start) / (end-start)
            scene.frame_set(frame)
            restore(rig, base)
            at = phase * (len(record['samples'])-1)
            a, b = int(at), min(int(at)+1, len(record['samples'])-1)
            t = at-a
            first, second = record['samples'][a], record['samples'][b]
            for name, angle in first['rotations'].items():
                value = -(angle + (second['rotations'][name]-angle)*t)
                value = (value + math.pi) % math.tau - math.pi
                limit = 2.3 if name.startswith('UpperArm') else 1.65
                if name == 'Hips':
                    limit = .5
                elif name in ('Torso', 'Head'):
                    limit = .55
                elif name.startswith('Foot'):
                    limit = .8
                value = max(-limit, min(limit, value))
                p = rig.pose.bones[name]
                rest = p.bone.matrix_local.to_3x3()
                rotation(p, base[name][0] @ (rest.inverted() @ Matrix.Rotation(value, 3, 'X') @ rest).to_quaternion())
            root = [first['root'][j] + (second['root'][j]-first['root'][j])*t-center[j] for j in range(2)]
            hips = rig.pose.bones['Hips']
            rest = hips.bone.matrix_local.to_3x3()
            hips.location = base['Hips'][1] + rest.inverted() @ Vector(
                (0, max(-.08, min(.08, -.35*root[0])), max(-.1, min(.16, .65*root[1]))))
            # Secondary bones follow the actual sampled body sway. The mounted Id
            # retains its authored seat and companions, including all idle offsets.
            sway = first['rotations']['Torso'] + (second['rotations']['Torso']-first['rotations']['Torso'])*t
            for name, factor in [('Hair', .3), ('Cape', .2), ('Tail', .4)]:
                if name in rig.pose.bones:
                    p = rig.pose.bones[name]
                    rest = p.bone.matrix_local.to_3x3()
                    rotation(p, base[name][0] @ (rest.inverted() @ Matrix.Rotation(sway*factor, 3, 'X') @ rest).to_quaternion())
            if slug == 'ui':
                ui_move.apply(rig, base, record, a, b, t,
                              lambda name, angle: ui_move.turn_from_base(rig, base, name, angle, rotation))
            bpy.context.view_layer.update()
            evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
            floor = min((evaluated.matrix_world @ v.co).z for v in evaluated.data.vertices)
            if floor < base_floor and slug not in ('id', 'ui'):
                hips.location += hips.bone.matrix_local.to_3x3().inverted() @ Vector((0, 0, base_floor-floor))
            if frame == start:
                first_pose = transforms(rig)
            if frame == end:
                restore(rig, first_pose)
            for p in rig.pose.bones:
                for prop in ['rotation_quaternion' if p.rotation_mode == 'QUATERNION' else 'rotation_euler', 'location', 'scale']:
                    p.keyframe_insert(data_path=prop, frame=frame, group=p.name)
        for c in curves(rig):
            for key in c.keyframe_points:
                if start <= key.co.x <= end:
                    key.interpolation = 'LINEAR'
        def pose_at(frame):
            scene.frame_set(frame)
            bpy.context.view_layer.update()
            return [v for p in rig.pose.bones for row in p.matrix for v in row]
        first_values = pose_at(start)
        seam = max(abs(a-b) for a, b in zip(first_values, pose_at(end)))
        variation = max(abs(a-b) for f in range(start+1, end) for a, b in zip(first_values, pose_at(f)))
        assert seam < .0001 and variation > .001, (slug, seam, variation)
        checks.append({'frames': [start, end], 'seam': seam, 'variation': variation})
    assert other_keys() == before, (slug, 'unrelated animation keys changed')
    assert {o.name: mesh_hash(o) for o in meshes} == geometry, (slug, 'mesh/weights/morphs changed')
    scene['move_source'] = 'Original battle Move; source periods restored at runtime'
    scene['move_seconds'] = record['duration']
    if slug == 'ui':
        scene['ui_move'] = 'Source Eru_Body squash/hop and frog limbs; seated rider; visual only'
    scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(blend))
    export_fbx(mesh, rig, fbx)
    manifest_path = ROOT / 'assets/characters/manifest.json'
    manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    item = next(item for item in manifest if item['slug'] == slug)
    item['move'] = {key: record[key] for key in ['clip', 'duration', 'source', 'sha256']}
    for key, path in [('blend', blend), ('model', fbx)]:
        item['sha256'][key] = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    reports = [item for item in reports if item['slug'] != slug]
    reports.append({'slug': slug, 'duration': record['duration'], 'mesh_sha256': geometry,
                    'unrelated_keys_preserved': True, 'clips': checks})
    report_path.write_text(json.dumps(reports, indent=2), encoding='utf-8')
    print('MOVE_ASSET_OK', slug, record['duration'], flush=True)
print('MOVE_ASSETS_COMPLETE', len(reports), flush=True)

