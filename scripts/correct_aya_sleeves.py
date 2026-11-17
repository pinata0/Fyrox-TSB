"""Match Aya's existing arm garments to her white kimono without rebuilding the model."""
import bpy
import hashlib
import json
import sys
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from export_fyrox_fbx import export_fbx

bpy.ops.wm.open_mainfile(filepath=str(ROOT / 'assets/characters/aya.blend'))
mesh = bpy.data.objects['aya_SkinnedMesh']
rig = bpy.data.objects['TSB_Rig']
scene = bpy.context.scene
groups = {g.index: g.name for g in mesh.vertex_groups}
geometry = [(tuple(v.co), [(g.group, g.weight) for g in v.groups]) for v in mesh.data.vertices]
white = next(i for i, m in enumerate(mesh.data.materials) if m.name.startswith('Color_fffaf4'))
counts = {'L': 0, 'R': 0}
for face in mesh.data.polygons:
    if not mesh.data.materials[face.material_index].name.startswith(('Color_bc7e72', 'Color_fffaf4')):
        continue
    for side in counts:
        if all(sum(g.weight for g in mesh.data.vertices[i].groups
                   if groups[g.group] in [f'UpperArm.{side}', f'Forearm.{side}']) > .99
               for i in face.vertices):
            face.material_index = white
            counts[side] += 1
            break
assert all(n > 100 for n in counts.values()), counts
assert geometry == [(tuple(v.co), [(g.group, g.weight) for g in v.groups]) for v in mesh.data.vertices]
assert len(rig.data.bones) == 20
scene.frame_set(0)
scene['aya_sleeves'] = 'White kimono sleeve materials; original brown hands and rig preserved.'
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / 'assets/characters/aya.blend'))
export_fbx(mesh, rig, ROOT / 'assets/characters/aya.fbx')
scene.cycles.samples = 16
for kind, resolution, location, target, scale in [
    ('preview', 512, (2.6, -4, 2.15), (0, 0, 1.1), 2.95),
    ('portrait', 256, (0, -5, 1.60), (-.06, 0, 1.60), 1.85),
]:
    scene.camera.location = location
    scene.camera.rotation_euler = (Vector(target) - scene.camera.location).to_track_quat('-Z', 'Y').to_euler()
    scene.camera.data.ortho_scale = scale
    scene.render.resolution_x = scene.render.resolution_y = resolution
    scene.render.filepath = str(ROOT / f'assets/characters/aya-{kind}.png')
    bpy.ops.render.render(write_still=True)
records_path = ROOT / 'assets/characters/manifest.json'
records = json.loads(records_path.read_text(encoding='utf-8'))
next(r for r in records if r['slug'] == 'aya')['sleeve_correction'] = 'White kimono sleeves; brown hands preserved; 2026-11-13'
records_path.write_text(json.dumps(records, ensure_ascii=False, indent=2), encoding='utf-8')
result = {'passed': True, 'recolored_faces': counts, 'geometry_and_skin_weights_unchanged': True,
          'bones': len(rig.data.bones), 'fbx_sha256': hashlib.sha256((ROOT / 'assets/characters/aya.fbx').read_bytes()).hexdigest()}
(ROOT / 'artifacts/greeting-sleeves/aya-validation.json').write_text(json.dumps(result, indent=2))
print('AYA_SLEEVES_OK', result, flush=True)
