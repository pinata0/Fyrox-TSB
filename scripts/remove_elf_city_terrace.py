"""Remove the city hall terrace from the saved scene, preserving manual edits.

blender --background --python-exit-code 1 --python scripts/remove_elf_city_terrace.py -- --render
"""
import hashlib
import json
import math
import shutil
import sys
from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from elf_city_layout import make_layout, OUT

ART = ROOT / 'artifacts/elf-city-terrace-removal'
ART.mkdir(parents=True, exist_ok=True)
source = OUT / 'elf-city.blend'
layout_path = OUT / 'layout.json'
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
original_hashes = {name: sha(OUT / name) for name in
                   ['elf-city.blend', 'elf-city.fbx', 'layout.json', 'build-report.json', 'preview.png']}
backup = ART / ('source-' + original_hashes[source.name][:12])
backup.mkdir(exist_ok=True)
for name in original_hashes:
    if not (backup / name).exists():
        shutil.copy2(OUT / name, backup / name)

old = json.loads(layout_path.read_text(encoding='utf-8'))
layout = make_layout()
key = lambda block: json.dumps(block, sort_keys=True)
new_keys = {key(block) for block in layout['blocks']}
old_keys = {key(block) for block in old['blocks']}
removed_blocks = [block for block in old['blocks'] if key(block) not in new_keys]
assert new_keys <= old_keys, 'Removal must not add or change unrelated collision blocks'
assert len(removed_blocks) == 18 and all('terrace' in b['name'].lower() for b in removed_blocks)

bpy.ops.wm.open_mainfile(filepath=str(source))
bpy.context.preferences.filepaths.file_preview_type = 'NONE'
if bpy.context.object and bpy.context.object.mode != 'OBJECT':
    bpy.ops.object.mode_set(mode='OBJECT')
scene = bpy.context.scene
bpy.context.view_layer.update()


def signature(obj):
    data = dict(matrix=[list(row) for row in obj.matrix_world],
                collections=sorted(c.name for c in obj.users_collection),
                hide_render=obj.hide_render, hide_viewport=obj.hide_viewport,
                properties=dict(obj.items()))
    if obj.type == 'MESH':
        data.update(vertices=[list(v.co) for v in obj.data.vertices],
                    faces=[list(p.vertices) for p in obj.data.polygons],
                    uv=[[list(v.uv) for v in layer.data] for layer in obj.data.uv_layers],
                    materials=[m.name for m in obj.data.materials])
    return hashlib.sha256(json.dumps(data, sort_keys=True, default=str).encode()).hexdigest()


removed_names = {b['name'] for b in removed_blocks}
step_positions = [Vector((b['center'][0], -b['center'][2], b['size'][1] + .012))
                  for b in removed_blocks if 'step' in b['name']]
removed = [o for o in scene.objects
           if o.name.rsplit('.', 1)[0] in removed_names
           or o.name.startswith('City hall balcony')
           or (o.name.startswith('Step nosing')
               and any((o.location - pos).length < .001 for pos in step_positions))]
assert len(removed) == 42, [o.name for o in removed]
assert sum(bool(o.get('collision')) for o in removed) == len(removed_blocks)
unchanged = {o.name: signature(o) for o in scene.objects if o not in removed}
removed_object_names = sorted(o.name for o in removed)
for obj in removed:
    bpy.data.objects.remove(obj, do_unlink=True)
bpy.context.view_layer.update()
assert all(signature(scene.objects[name]) == digest for name, digest in unchanged.items())
assert not any('terrace' in o.name.lower() or o.name.startswith('City hall balcony') for o in scene.objects)
staged = ART / source.name
bpy.ops.wm.save_as_mainfile(filepath=str(staged), relative_remap=False)
meshes = [o for o in scene.objects if o.type == 'MESH']
stats = dict(slug=layout['slug'], objects=len(meshes),
             vertices=sum(len(o.data.vertices) for o in meshes),
             triangles=sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in meshes),
             collision_blocks=len(layout['blocks']), stairs=len(layout['stairs']),
             materials=sum('srgb' in m for m in bpy.data.materials))
print('TERRACE_REMOVED', len(removed), 'objects;', len(unchanged), 'other objects preserved', flush=True)

# Match the builder's spatial/material batching and Fyrox sRGB export.
groups = {}
for obj in meshes:
    center = sum((obj.matrix_world @ Vector(c) for c in obj.bound_box), Vector()) / 8
    group = (math.floor(center.x / 20), math.floor(-center.y / 20), obj.data.materials[0].name)
    groups.setdefault(group, []).append(obj)
exports = []
for group, objects in groups.items():
    bpy.ops.object.select_all(action='DESELECT')
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    if len(objects) > 1:
        bpy.ops.object.join()
    obj = bpy.context.object
    obj.name = 'ElfCity_%s_%s_%s' % group
    exports.append(obj)
for mat in bpy.data.materials:
    if 'srgb' in mat:
        rgba = tuple(v / 255 for v in mat['srgb']) + (1,)
        mat.diffuse_color = rgba
        mat.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value = rgba
bpy.ops.object.select_all(action='DESELECT')
for obj in exports:
    obj.select_set(True)
bpy.ops.export_scene.fbx(filepath=str(ART / 'elf-city.fbx'), use_selection=True,
                         object_types={'MESH'}, apply_unit_scale=True, axis_forward='-Z', axis_up='Y',
                         bake_anim=False, path_mode='AUTO')
stats['export_chunks'] = len(exports)
assert all(sha(OUT / name) == digest for name, digest in original_hashes.items()), 'Source changed; result staged only'
shutil.copy2(staged, source)
shutil.copy2(ART / 'elf-city.fbx', OUT / 'elf-city.fbx')
layout_path.write_text(json.dumps(layout, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
(OUT / 'build-report.json').write_text(json.dumps(stats, indent=2) + '\n', encoding='utf-8')
report = dict(removed_objects=removed_object_names, removed_blocks=removed_blocks,
              unchanged_objects_verified=len(unchanged), stats=stats,
              files={name: sha(OUT / name) for name in ['elf-city.blend', 'elf-city.fbx', 'layout.json']})
(ART / 'verification.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('ELF_TERRACE_REMOVAL_OK', json.dumps(stats), flush=True)

if '--render' in sys.argv:
    bpy.ops.wm.open_mainfile(filepath=str(source))
    scene = bpy.context.scene
    scene.camera = bpy.data.objects['Civic preview']
    scene.render.filepath = str(ART / 'civic-preview.png')
    bpy.ops.render.render(write_still=True)
    shutil.copy2(ART / 'civic-preview.png', OUT / 'preview.png')
    print('ELF_TERRACE_PREVIEW_OK', flush=True)
