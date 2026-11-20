"""Blender QA for Id's split pod, preserved face and source-timed animations.

Run after bake_id_skills.py and export_meshy_morphs.py. The first bake records
the original assets in artifacts/id-skills/before for the geometry comparison.
"""
import bpy
import hashlib
import json
import math
from pathlib import Path
from mathutils import Quaternion

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/characters'
WORK = ROOT / 'artifacts/id-skills'


def geometry():
    mesh = bpy.data.objects['id_SkinnedMesh'].data
    return {
        'vertices': [tuple(v.co) for v in mesh.vertices],
        'faces': [tuple(p.vertices) for p in mesh.polygons],
        'keys': {k.name: [tuple(v.co) for v in k.data]
                 for k in mesh.shape_keys.key_blocks},
    }


def surface_faces(objects):
    """Compare actual coordinates and UV corners, independent of reindexing."""
    result = []
    for obj in objects:
        mesh = obj.data
        uv = mesh.uv_layers.active.data
        for face in mesh.polygons:
            corners = [tuple(mesh.vertices[mesh.loops[i].vertex_index].co) + tuple(uv[i].uv)
                       for i in face.loop_indices]
            rotations = [tuple(corners[i:] + corners[:i]) for i in range(len(corners))]
            result.append((face.material_index, min(rotations)))
    return sorted(result)


bpy.ops.wm.open_mainfile(filepath=str(WORK / 'before/id.blend'))
before_body = geometry()
before_pod = surface_faces([bpy.data.objects['Crescent']])
before_posture = {}
for frame in range(154):
    bpy.context.scene.frame_set(frame)
    before_posture[frame] = {n: bpy.data.objects['TSB_Rig'].pose.bones[n].rotation_quaternion.copy()
                            for n in ['Torso', 'Head']}
bpy.ops.wm.open_mainfile(filepath=str(ASSETS / 'id.blend'))
assert geometry() == before_body, 'Rider geometry or face keys changed'
rig = bpy.data.objects['TSB_Rig']
parts = [o for o in bpy.context.scene.objects if o.name.startswith('IdPodMesh.')]
assert len(parts) == 6
assert surface_faces(parts) == before_pod, 'Pod faces, winding, materials or UVs changed'
assert len(rig.data.bones) == 69
assert len([b for b in rig.data.bones if b.name.startswith('IdHair.')]) == 9
assert not set(o.name for o in bpy.context.scene.objects).intersection(b.name for b in rig.data.bones)
scene = bpy.context.scene
assert scene.frame_end == 826 and scene.render.fps == 30
clips = json.loads(scene['clips'])
assert clips['IdSkill'] == [531, 639] and clips['IdUltimate'] == [666, 826]
for frame, original in before_posture.items():
    scene.frame_set(frame)
    for name, angle in [('Torso', -.28), ('Head', .12)]:
        expected = Quaternion((1, 0, 0), angle) @ original[name]
        actual = rig.pose.bones[name].rotation_quaternion
        assert min(sum(abs(a-b) for a, b in zip(actual, expected)),
                   sum(abs(a+b) for a, b in zip(actual, expected))) < 1e-4, (frame, name)

def basis(name):
    return tuple(v for row in rig.pose.bones[name].matrix_basis for v in row)

scene.frame_set(156)
still = {n: basis(n) for n in ['Hips', 'Torso', 'Head', 'UpperArm.L', 'UpperArm.R']}
for frame in [160, 168, 180]:
    scene.frame_set(frame)
    for name, original in still.items():
        assert max(abs(a-b) for a, b in zip(basis(name), original)) < 1e-5, name

changing = ['Mount', 'IdPod.Bow', 'IdPod.Back', 'IdPod.Crown', 'IdHair.L.3', 'IdHair.C.3', 'IdHair.R.3']
motion = {name: [] for name in changing}
for frame in range(666, 827, 4):
    scene.frame_set(frame)
    for bone in rig.pose.bones:
        assert all(math.isfinite(v) for row in bone.matrix for v in row), (frame, bone.name)
    for name in changing:
        motion[name].append(basis(name))
for name, samples in motion.items():
    assert max(max(abs(a-b) for a, b in zip(sample, samples[0])) for sample in samples) > 0.001, name

report = json.loads((WORK / 'asset-report.json').read_text(encoding='utf-8'))
report.update(bones=len(rig.data.bones), body_vertices_unchanged=len(before_body['vertices']),
              facial_keys_unchanged=len(before_body['keys'])-1,
              pod_faces_uv_materials_preserved=len(before_pod),
              stationary_attack=True, independently_animated=changing)
hashes = {name: hashlib.sha256((ASSETS / name).read_bytes()).hexdigest()
          for name in ['id.blend', 'id.fbx', 'id.morphs.json']}
report['sha256'] = hashes
(WORK / 'asset-verification.json').write_text(json.dumps(report, indent=2), encoding='utf-8')

# Update only Id's records, preserving other characters and historical provenance.
rig_path = ASSETS / 'id-rig.json'
record = json.loads(rig_path.read_text(encoding='utf-8'))
record['bones'] = 69
record['clips'].update({n: clips[n] for n in ['IdSkill', 'IdPulse', 'IdUltimate']})
record['companions'] = [o.name for o in parts] + ['Nata']
record['id_skills'] = report
rig_path.write_text(json.dumps(record, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
manifest_path = ASSETS / 'manifest.json'
manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
record = next(r for r in manifest if r['slug'] == 'id')
record['bones'] = 69
record['clips'].update({n: clips[n] for n in ['IdSkill', 'IdPulse', 'IdUltimate']})
record['companions'] = [o.name for o in parts] + ['Nata']
record['sha256'].update(blend=hashes['id.blend'], model=hashes['id.fbx'], morph_data=hashes['id.morphs.json'])
record['id_skills'] = report
manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print('ID_ASSET_VERIFIED', json.dumps(report), flush=True)
