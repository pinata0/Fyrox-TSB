"""Render and validate the bodysuit correction in Ed's existing seated rig."""
import bpy
import json
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/id-bodysuit-correction'
bpy.ops.wm.open_mainfile(filepath=str(ROOT / 'assets/characters/id.blend'))
scene = bpy.context.scene
rig = bpy.data.objects['TSB_Rig']
mesh = bpy.data.objects['id_SkinnedMesh']
scene.frame_set(0)
scene['id_outfit_correction'] = 'Fitted sleeveless white/black one-piece bodysuit, crossed neck straps, bare arms, white wrist bands, charcoal thigh-high stockings with white side piping and white low shoes. Based on user-provided costume references; crescent pod and seated rig preserved.'
assert len(rig.data.bones) == 20
assert all(abs(sum(g.weight for g in v.groups) - 1) < .0001 for v in mesh.data.vertices)
assert bytes(bpy.data.objects['Turnaround_REFERENCE_only'].data.packed_file.data) == (ROOT / 'assets/turnarounds/id.png').read_bytes()
gray = next(i for i, m in enumerate(mesh.data.materials) if m.name.startswith('Color_393743'))
skin = next(i for i, m in enumerate(mesh.data.materials) if m.name.startswith('Color_ffe2d1'))
panels = {}
for bone in ['Thigh.L', 'Thigh.R', 'Shin.L', 'Shin.R', 'UpperArm.L', 'UpperArm.R', 'Forearm.L', 'Forearm.R']:
    index = mesh.vertex_groups[bone].index
    expected_material = skin if 'Arm' in bone or 'Forearm' in bone else gray
    polys = [p for p in mesh.data.polygons if p.material_index == expected_material and all(any(g.group == index and g.weight > .99 for g in mesh.data.vertices[v].groups) for v in p.vertices)]
    assert len(polys) > 10, (bone, 'missing bare arm or dark stocking')
    panels[bone] = len(polys)
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / 'assets/characters/id.blend'))
scene.camera.location = (0, -5, 1.58)
scene.camera.rotation_euler = (Vector((0, 0, 1.58)) - scene.camera.location).to_track_quat('-Z', 'Y').to_euler()
scene.camera.data.ortho_scale = 1.55
scene.render.resolution_x = 256
scene.render.resolution_y = 256
scene.cycles.samples = 16
scene.render.filepath = str(ROOT / 'assets/characters/id-portrait.png')
bpy.ops.render.render(write_still=True)
for label, location in [('front', (0, -5, 1.6)), ('back', (0, 5, 1.6))]:
    scene.camera.location = location
    scene.camera.rotation_euler = (Vector((0, 0, 1.1)) - scene.camera.location).to_track_quat('-Z', 'Y').to_euler()
    scene.camera.data.ortho_scale = 2.65
    scene.render.resolution_x = 512
    scene.render.resolution_y = 512
    scene.render.filepath = str(OUT / f'model-{label}.png')
    bpy.ops.render.render(write_still=True)
progress = json.loads((ROOT / 'assets/characters/build-progress.json').read_text())[0]
assert progress['slug'] == 'id'
records = json.loads((ROOT / 'assets/characters/manifest.json').read_text(encoding='utf-8'))
for entry in records:
    if entry['slug'] == 'id':
        entry.update(progress)
        entry.update(portrait='assets/characters/id-portrait.png', correction=scene['id_outfit_correction'])
(ROOT / 'assets/characters/manifest.json').write_text(json.dumps(records, ensure_ascii=False, indent=2), encoding='utf-8')
(OUT / 'model-validation.json').write_text(json.dumps(dict(passed=True, bones=20, vertices=len(mesh.data.vertices), bodysuit_material_faces_by_bone=panels, packed_guide_matches=True), indent=2), encoding='utf-8')
print('ID_BODYSUIT_MODEL_VERIFIED', progress['vertices'], progress['triangles'], panels, flush=True)
