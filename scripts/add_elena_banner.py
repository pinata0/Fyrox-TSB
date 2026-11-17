"""Patch the saved city without rebuilding unrelated hand-edited geometry.

blender --background --python-exit-code 1 --python scripts/add_elena_banner.py -- --render
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
from elf_city_banner import add_banner, is_replaced, TEXTURE

OUT = ROOT / 'assets/maps/elf-city'
ART = ROOT / 'artifacts/elena-banner'
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()


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


ART.mkdir(parents=True, exist_ok=True)
files = ['elf-city.blend', 'elf-city.fbx', 'layout.json', 'build-report.json', 'preview.png']
original = {name: sha(OUT / name) for name in files}
backup = ART / ('source-' + original['elf-city.blend'][:12])
backup.mkdir(exist_ok=True)
for name in files:
    if not (backup / name).exists():
        shutil.copy2(OUT / name, backup / name)

bpy.ops.wm.open_mainfile(filepath=str(OUT / 'elf-city.blend'))
bpy.context.preferences.filepaths.file_preview_type = 'NONE'
scene = bpy.context.scene
if bpy.context.object and bpy.context.object.mode != 'OBJECT':
    bpy.ops.object.mode_set(mode='OBJECT')
bpy.context.view_layer.update()
unchanged = {obj.name: signature(obj) for obj in scene.objects if not is_replaced(obj)}
replaced = [obj.name for obj in scene.objects if is_replaced(obj)]
assert replaced, 'Expected existing municipal panel or a previously installed banner'
assert not any(obj.get('collision') for obj in scene.objects if is_replaced(obj))
added = add_banner()
added_names = [obj.name for obj in added]
bpy.context.view_layer.update()
assert len(added) == 4
assert all(signature(scene.objects[name]) == digest for name, digest in unchanged.items())
for obj in added:
    assert obj.data.uv_layers
    assert all(face.area > 1e-10 for face in obj.data.polygons)
artwork = next(obj for obj in added if obj.name.endswith('full body'))
image = bpy.data.images['elena-banner.png']
assert abs(artwork.dimensions.y / artwork.dimensions.z - image.size[0] / image.size[1]) < .0001
assert all(p.normal.x < -.99 for p in artwork.data.polygons), 'Portrait must face the plaza'
staged = ART / 'elf-city.blend'
bpy.ops.wm.save_as_mainfile(filepath=str(staged), relative_remap=False)
meshes = [obj for obj in scene.objects if obj.type == 'MESH']
layout = json.loads((OUT / 'layout.json').read_text(encoding='utf-8'))
stats = dict(slug='elf-city', objects=len(meshes),
             vertices=sum(len(obj.data.vertices) for obj in meshes),
             triangles=sum(sum(len(p.vertices) - 2 for p in obj.data.polygons) for obj in meshes),
             collision_blocks=len(layout['blocks']), stairs=len(layout['stairs']),
             materials=sum('srgb' in mat for mat in bpy.data.materials),
             elena_banner_texture='elena-banner.png')

# Same spatial/material batching as build_elf_city.py.
groups = {}
for obj in meshes:
    center = sum((obj.matrix_world @ Vector(c) for c in obj.bound_box), Vector()) / 8
    key = (math.floor(center.x / 20), math.floor(-center.y / 20), obj.data.materials[0].name)
    groups.setdefault(key, []).append(obj)
exports = []
for key, objects in groups.items():
    bpy.ops.object.select_all(action='DESELECT')
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    if len(objects) > 1:
        bpy.ops.object.join()
    obj = bpy.context.object
    obj.name = 'ElfCity_%s_%s_%s' % key
    exports.append(obj)
for mat in bpy.data.materials:
    if 'srgb' in mat:
        rgba = tuple(v / 255 for v in mat['srgb']) + (1,)
        mat.diffuse_color = rgba
        mat.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value = rgba
bpy.data.images['elena-banner.png'].filepath = str(TEXTURE)
bpy.ops.object.select_all(action='DESELECT')
for obj in exports:
    obj.select_set(True)
bpy.ops.export_scene.fbx(filepath=str(ART / 'elf-city.fbx'), use_selection=True,
                         object_types={'MESH'}, apply_unit_scale=True, axis_forward='-Z', axis_up='Y',
                         bake_anim=False, path_mode='STRIP')
stats['export_chunks'] = len(exports)
assert all(sha(OUT / name) == digest for name, digest in original.items()), 'Source changed while staging'
shutil.copy2(staged, OUT / 'elf-city.blend')
shutil.copy2(ART / 'elf-city.fbx', OUT / 'elf-city.fbx')
(OUT / 'build-report.json').write_text(json.dumps(stats, indent=2) + '\n', encoding='utf-8')
report = dict(replaced_objects=replaced, added_objects=added_names,
              unchanged_objects_verified=len(unchanged), collision_layout_unchanged=original['layout.json'] == sha(OUT / 'layout.json'),
              source_backup=str(backup.relative_to(ROOT)), stats=stats,
              files={name: sha(OUT / name) for name in ['elf-city.blend', 'elf-city.fbx', 'layout.json', 'elena-banner.png']})
(ART / 'verification.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('ELENA_BANNER_OK', len(unchanged), 'unrelated objects preserved;', json.dumps(stats), flush=True)

if '--render' in sys.argv:
    bpy.ops.wm.open_mainfile(filepath=str(OUT / 'elf-city.blend'))
    scene = bpy.context.scene
    scene.camera = bpy.data.objects['Civic preview']
    scene.render.filepath = str(ART / 'civic-preview.png')
    bpy.ops.render.render(write_still=True)
    shutil.copy2(ART / 'civic-preview.png', OUT / 'preview.png')
    print('ELENA_PREVIEW_OK', flush=True)
