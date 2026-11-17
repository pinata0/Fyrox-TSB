"""Bake Vivi's two source attacks while preserving all meshes and unrelated clips.
Run with Blender --background --python-exit-code 1 --python scripts/bake_vivi_combo.py.
"""
import hashlib
import json
import math
import sys
from pathlib import Path
import bpy
from mathutils import Matrix, Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from export_fyrox_fbx import export_fbx

records = json.loads((ROOT / 'assets/characters/attack/vivi-combo.json').read_text())
path = ROOT / 'assets/characters/vivi.blend'
bpy.ops.wm.open_mainfile(filepath=str(path))
# Keep explicit filenames: Blender path remapping must never turn a texture into a directory.
for image in bpy.data.images:
    filename = 'vivi-meshy-basecolor.png' if image.name == 'texture_0' else image.name + '.png'
    if (path.parent / filename).is_file():
        image.filepath = '//' + filename
rig = bpy.data.objects['TSB_Rig']
body = bpy.data.objects['vivi_SkinnedMesh']
meshes = [o for o in bpy.data.objects if o.type == 'MESH' and any(m.type == 'ARMATURE' and m.object == rig for m in o.modifiers)]
scene = bpy.context.scene
curves = rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves

def geometry():
    h = hashlib.sha256()
    for mesh in sorted(meshes, key=lambda m: m.name):
        for v in mesh.data.vertices:
            h.update(repr((tuple(v.co), [(g.group, g.weight) for g in v.groups])).encode())
        for p in mesh.data.polygons:
            h.update(repr((tuple(p.vertices), p.material_index)).encode())
    return h.hexdigest()

def other_keys():
    result = {}
    for c in curves:
        keys = [tuple(k.co) for k in c.keyframe_points if not 156 <= k.co.x <= 180 and k.co.x < 531]
        if keys:
            result[(c.data_path, c.array_index)] = keys
    return result

before, mesh_hash = other_keys(), geometry()
scene.frame_set(0)
base = {p.name: (p.rotation_quaternion.copy() if p.rotation_mode == 'QUATERNION' else p.rotation_euler.to_quaternion(), p.location.copy(), p.scale.copy()) for p in rig.pose.bones}

def rotate(p, q):
    if p.rotation_mode == 'QUATERNION':
        if p.rotation_quaternion.dot(q) < 0:
            q = -q
        p.rotation_quaternion = q
    else:
        p.rotation_euler = q.to_euler('XYZ', p.rotation_euler)

def smooth(t):
    t = max(0, min(1, t))
    return t * t * (3 - 2 * t)

checkpoints = []
for record, start in zip(records, [156, 531]):
    for frame in range(start, start + 25):
        phase = (frame - start) / 24
        scene.frame_set(frame)
        for p in rig.pose.bones:
            q, loc, scale = base[p.name]
            rotate(p, q)
            p.location, p.scale = loc, scale
        at = phase * (len(record['samples']) - 1)
        a = int(at)
        b = min(a + 1, len(record['samples']) - 1)
        t = at - a
        first, second = record['samples'][a], record['samples'][b]
        weight = smooth(phase / .08) * smooth((1 - phase) / .14)
        for name, angle in first['rotations'].items():
            value = -(angle + (second['rotations'][name] - angle) * t) * weight
            limit = 1.8 if name.startswith(('Forearm', 'Hand')) else 2.0
            if name == 'Hips': limit = .24
            elif name in ('Torso', 'Head'): limit = .55
            elif name.startswith(('Thigh', 'Shin', 'Foot')): limit = .5
            value = max(-limit, min(limit, value))
            p = rig.pose.bones[name]
            rest = p.bone.matrix_local.to_3x3()
            rotate(p, base[name][0] @ (rest.inverted() @ Matrix.Rotation(value, 3, 'X') @ rest).to_quaternion())
        root = [(first['root'][j] + (second['root'][j] - first['root'][j]) * t) * weight for j in range(2)]
        hips = rig.pose.bones['Hips']
        rest = hips.bone.matrix_local.to_3x3()
        hips.location = base['Hips'][1] + rest.inverted() @ Vector((0, -.28 * math.tanh(root[0]), max(-.12, min(.18, root[1] * .45))))
        bpy.context.view_layer.update()
        evaluated = body.evaluated_get(bpy.context.evaluated_depsgraph_get())
        floor = min((evaluated.matrix_world @ v.co).z for v in evaluated.data.vertices)
        if floor < .012 and 0 < phase < 1:
            hips.location += rest.inverted() @ Vector((0, 0, .012 - floor))
        for p in rig.pose.bones:
            p.keyframe_insert(data_path='rotation_quaternion' if p.rotation_mode == 'QUATERNION' else 'rotation_euler', frame=frame, group=p.name)
            p.keyframe_insert(data_path='location', frame=frame, group=p.name)
    for curve in curves:
        for key in curve.keyframe_points:
            if start <= key.co.x <= start + 24:
                key.interpolation = 'LINEAR'
    poses = []
    for phase in [0, .2, .4, .6, .8, 1]:
        frame = start + phase * 24
        scene.frame_set(int(frame), subframe=frame-int(frame))
        bpy.context.view_layer.update()
        poses.append({name: list(rig.matrix_world @ rig.pose.bones[name].head) for name in ['Hips', 'Head', 'Hand.L', 'Hand.R']})
    checkpoints.append({'clip': record['clip'], 'poses': poses})
assert geometry() == mesh_hash
assert other_keys() == before, 'unrelated animation keys changed'
scene.frame_end = 555
scene['attack_source'] = 'Vivi Attack1_1 x2 -> Attack2_1; independent empty hands and levitating blades'
scene.frame_set(0)
bpy.context.preferences.filepaths.save_version = 0
bpy.ops.wm.save_as_mainfile(filepath=str(path), relative_remap=False)
for image in bpy.data.images:
    if image.source == 'FILE':
        assert Path(bpy.path.abspath(image.filepath)).is_file(), (image.name, image.filepath)
export_fbx(body, rig, path.with_suffix('.fbx'), [m for m in meshes if m != body])
(ROOT / 'assets/characters/attack/vivi-checkpoints.json').write_text(json.dumps(checkpoints))
manifest_path = ROOT / 'assets/characters/manifest.json'
manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
vivi = next(r for r in manifest if r['slug'] == 'vivi')
vivi['clips']['Attack2'] = [531, 555]
for key in ['blend', 'model']:
    vivi['sha256'][key] = hashlib.sha256((ROOT / vivi[key]).read_bytes()).hexdigest()
manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
rig_path = ROOT / 'assets/characters/vivi-rig.json'
rig_record = json.loads(rig_path.read_text(encoding='utf-8'))
rig_record['clips']['Attack2'] = [531, 555]
rig_path.write_text(json.dumps(rig_record, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
(ROOT / 'artifacts/vivi-swordplay/asset-verification.json').write_text(json.dumps({'mesh_sha256': mesh_hash, 'meshes_preserved': len(meshes), 'unrelated_keys_preserved': True, 'clips': checkpoints}, indent=2))
print('VIVI_COMBO_ASSETS_OK', len(meshes), flush=True)
