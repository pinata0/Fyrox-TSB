"""Bake sampled Spawn into the old selection interval (456..528) on all current rigs.

Run extract_spawn_motions.cjs, then Blender --background --python-exit-code 1
--python scripts/bake_spawn_motions.py [-- slug ...]. Source timing is restored by
Motion::Spawn in both the world and selection preview. No other clip is rebuilt.
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

OUT = ROOT / 'artifacts/spawn-motions'
BACKUP = OUT / 'before/assets/characters'
BACKUP.mkdir(parents=True, exist_ok=True)
DATA = json.loads((ROOT / 'assets/characters/spawn/motions.json').read_text(encoding='utf-8'))


def curves(rig):
    action = rig.animation_data.action
    return action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves


def mesh_hash(mesh):
    h = hashlib.sha256()
    for v in mesh.data.vertices:
        h.update(repr((tuple(v.co), [(g.group, g.weight) for g in v.groups])).encode())
    for p in mesh.data.polygons:
        h.update(repr((tuple(p.vertices), p.material_index)).encode())
    return h.hexdigest()


def rotation(p, q):
    if p.rotation_mode == 'QUATERNION':
        if p.rotation_quaternion.dot(q) < 0:
            q = -q
        p.rotation_quaternion = q
    else:
        p.rotation_euler = q.to_euler('XYZ', p.rotation_euler)


def smooth(t):
    t = max(0, min(1, t))
    return t*t*(3-2*t)


only = sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
report_path = OUT / 'asset-verification.json'
reports = json.loads(report_path.read_text(encoding='utf-8')) if only and report_path.exists() else []
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
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == 'MESH'
              and any(m.type == 'ARMATURE' and m.object == rig for m in obj.modifiers)]
    scene = bpy.context.scene

    def other_keys():
        return [(c.data_path, c.array_index, [(tuple(p.co), p.interpolation) for p in c.keyframe_points
                 if not 456 <= p.co.x <= 528]) for c in curves(rig)
                if any(not 456 <= p.co.x <= 528 for p in c.keyframe_points)]

    before = other_keys()
    geometry = {obj.name: mesh_hash(obj) for obj in meshes}
    scene.frame_set(0)
    base = {p.name: (p.rotation_quaternion.copy() if p.rotation_mode == 'QUATERNION'
                    else p.rotation_euler.to_quaternion(), p.location.copy(), p.scale.copy())
            for p in rig.pose.bones}
    bpy.context.view_layer.update()
    evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    base_floor = min((evaluated.matrix_world @ v.co).z for v in evaluated.data.vertices)
    checkpoints = []
    for frame in range(456, 529):
        phase = (frame - 456) / 72
        scene.frame_set(frame)
        for p in rig.pose.bones:
            q, loc, scale = base[p.name]
            rotation(p, q)
            p.location = loc
            p.scale = scale
        at = phase * (len(record['samples']) - 1)
        a = int(at)
        b = min(a+1, len(record['samples'])-1)
        t = at-a
        first, second = record['samples'][a], record['samples'][b]
        # Only the final 0.12 s settles onto this model's own idle pose.
        weight = smooth((1-phase) * record['duration'] / .12)
        for name, angle in first['rotations'].items():
            raw = -(angle + (second['rotations'][name] - angle)*t)
            spinning = name == 'Hips' or name.startswith('UpperArm')
            if not spinning:
                raw = (raw + math.pi) % math.tau - math.pi
            # Completed turns settle to their equivalent idle rotation without
            # unwinding the whole spin during the last fraction of a second.
            turns = round(raw / math.tau) * math.tau
            value = raw * weight + turns * (1-weight)
            # Flat sprites can rotate a wrist through itself; retain the gesture
            # while keeping solid 3D limbs within a useful range.
            limit = 2.5 if name.startswith(('UpperArm', 'Forearm', 'Hand')) else 1.2
            if name == 'Head':
                limit = .8
            if not spinning or name != 'Hips' and abs(raw) < math.pi:
                value = max(-limit, min(limit, value))
            p = rig.pose.bones[name]
            rest = p.bone.matrix_local.to_3x3()
            delta = rest.inverted() @ Matrix.Rotation(value, 3, 'X') @ rest
            rotation(p, base[name][0] @ delta.to_quaternion())
        root = [(first['root'][j] + (second['root'][j]-first['root'][j])*t)*weight for j in range(2)]
        hips = rig.pose.bones['Hips']
        rest = hips.bone.matrix_local.to_3x3()
        hips.location = base['Hips'][1] + rest.inverted() @ Vector(
            (0, -.35*math.tanh(root[0]), max(-.25, min(.6, root[1]*.65))))
        bpy.context.view_layer.update()
        evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        floor = min((evaluated.matrix_world @ v.co).z for v in evaluated.data.vertices)
        if floor < base_floor:
            hips.location += rest.inverted() @ Vector((0, 0, base_floor-floor))
        for p in rig.pose.bones:
            p.keyframe_insert(data_path='rotation_quaternion' if p.rotation_mode == 'QUATERNION'
                              else 'rotation_euler', frame=frame, group=p.name)
            p.keyframe_insert(data_path='location', frame=frame, group=p.name)
            p.keyframe_insert(data_path='scale', frame=frame, group=p.name)
        if frame in [456, 474, 492, 510, 528]:
            bpy.context.view_layer.update()
            checkpoints.append({name: list(rig.matrix_world @ rig.pose.bones[name].head)
                                for name in ['Hips', 'Head', 'Hand.L', 'Hand.R']})
    for c in curves(rig):
        for key in c.keyframe_points:
            if 456 <= key.co.x <= 528:
                key.interpolation = 'LINEAR'
    assert other_keys() == before, (slug, 'unrelated animation keys changed')
    assert {obj.name: mesh_hash(obj) for obj in meshes} == geometry, (slug, 'mesh/weights changed')
    scene['spawn_source'] = 'Original battle Spawn: 30 Hz samples retargeted into frames 456-528'
    scene['spawn_seconds'] = record['duration']
    scene['selection_gesture'] = 'Original battle Spawn'
    scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(blend))
    export_fbx(mesh, rig, fbx)
    manifest_path = ROOT / 'assets/characters/manifest.json'
    manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    item = next(item for item in manifest if item['slug'] == slug)
    item['clips'].pop('Select', None)
    item['clips']['Spawn'] = [456, 528]
    item['selection_gesture'] = 'Original battle Spawn'
    item['spawn'] = {key: record[key] for key in ['clip', 'duration', 'source', 'sha256']}
    for key, path in [('blend', blend), ('model', fbx)]:
        item['sha256'][key] = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    reports = [item for item in reports if item['slug'] != slug]
    reports.append({'slug': slug, 'duration': record['duration'], 'mesh_sha256': geometry,
                    'unrelated_keys_preserved': True, 'checkpoints': checkpoints})
    reports.sort(key=lambda item: next(r['id'] for r in DATA if r['slug'] == item['slug']))
    report_path.write_text(json.dumps(reports, indent=2), encoding='utf-8')
    print('SPAWN_ASSET_OK', slug, record['duration'], flush=True)
print('SPAWN_ASSETS_COMPLETE', len(reports), flush=True)
