"""Replace saved map lettering without rebuilding manually edited scenery.

blender --background --python-exit-code 1 --python scripts/update_map_fonts.py
"""
import hashlib
import json
import math
import shutil
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from map_lettering import add_direction_arrow, font_body
OUT = ROOT / 'assets/maps/elf-city'
ART = ROOT / 'artifacts/map-font-update'
FONT = 'assets/fonts/Hungeul.otf'
# Older saved scenes have already converted text to meshes and retain no body.
LEGACY_TEXT = {
    'Transit hall sign': ('모나티엄   환승 정비고', 1.0),
    'Transit subtitle': ('MONATIUM   /   TRANSIT 01', .28),
    'Train serial': ('M-041', .36),
    'Civic title': ('모나티엄 시청', .53),
    'Vending header': ('BLOCK 24', .17),
    'Cafe neon sign': ('밤샘 연구소  /  COFFEE', .64),
    'PC cafe sign': ('민트 PC  /  24H', .7),
    'PC cafe wayfinding': ('측면 통로  →', .25),
    'Studio neon': ('NIGHT SHIFT / 041', .43),
    'Wayfinding': ('A  환승 정비고  ↑', .4),
    'Wayfinding.001': ('B  시청 광장  →', .4),
    'Wayfinding.002': ('시청 광장  ↑', .4),
    'Wayfinding.003': ('WELCOME / MONATIUM', .4),
    'Cargo serial': ('M / 041', .24),
    'Spawn label': ('MONATIUM / SAFE ZONE', .4),
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


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


def text_spec(obj):
    if 'map_text' in obj:
        return obj['map_text'], obj['map_font_size']
    return LEGACY_TEXT.get(obj.name, LEGACY_TEXT.get(obj.name.rsplit('.', 1)[0]))


ART.mkdir(parents=True, exist_ok=True)
files = ['elf-city.blend', 'elf-city.fbx', 'layout.json', 'build-report.json', 'preview.png']
original_hashes = {name: sha(OUT / name) for name in files}
backup = ART / ('source-' + original_hashes['elf-city.blend'][:12])
backup.mkdir(exist_ok=True)
for name in files:
    if not (backup / name).exists():
        shutil.copy2(OUT / name, backup / name)

bpy.ops.wm.open_mainfile(filepath=str(OUT / 'elf-city.blend'))
bpy.context.preferences.filepaths.file_preview_type = 'NONE'
scene = bpy.context.scene
bpy.context.view_layer.update()
targets = [obj for obj in scene.objects if obj.type == 'MESH' and text_spec(obj)]
assert len(targets) == 22, f'Unexpected map lettering inventory: {len(targets)}'
print('MAP_FONT_INVENTORY', len(targets), flush=True)
unchanged = {obj.name: signature(obj) for obj in scene.objects if obj not in targets}
font = bpy.data.fonts.load(str(ROOT / FONT))
records = []
for obj in targets:
    body, size = text_spec(obj)
    transform = obj.matrix_world.copy()
    old_mesh = obj.data
    curve = bpy.data.curves.new(obj.name + ' Hungeul', 'FONT')
    curve.body = font_body(body)
    curve.font = font
    curve.size = size
    curve.align_x = 'CENTER'
    curve.align_y = 'CENTER'
    curve.extrude = .008
    curve.resolution_u = 3
    temp = bpy.data.objects.new('Font replacement', curve)
    scene.collection.objects.link(temp)
    bpy.ops.object.select_all(action='DESELECT')
    temp.select_set(True)
    bpy.context.view_layer.objects.active = temp
    bpy.ops.object.convert(target='MESH')
    mesh = temp.data
    add_direction_arrow(mesh, body, size)
    # Apply the same cleanup and planar UVs as the map builder.
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=.000001)
    bmesh.ops.dissolve_degenerate(bm, edges=list(bm.edges), dist=.000001)
    bmesh.ops.triangulate(bm, faces=list(bm.faces))
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bad = [face for face in bm.faces if face.calc_area() < 1e-10]
    if bad:
        bmesh.ops.delete(bm, geom=bad, context='FACES')
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()
    uv = mesh.uv_layers.new(name='Planar UV')
    for face in mesh.polygons:
        axis = max(range(3), key=lambda i: abs(face.normal[i]))
        axes = [i for i in range(3) if i != axis]
        for li in face.loop_indices:
            co = mesh.vertices[mesh.loops[li].vertex_index].co
            uv.data[li].uv = (co[axes[0]] * .25, co[axes[1]] * .25)
    for material in old_mesh.materials:
        mesh.materials.append(material)
    obj.data = mesh
    bpy.data.objects.remove(temp, do_unlink=True)
    obj['map_text'] = body
    obj['map_font'] = FONT
    obj['map_font_size'] = size
    assert obj.matrix_world == transform
    assert mesh.vertices and mesh.polygons and all(p.area > 1e-10 for p in mesh.polygons)
    if not old_mesh.users:
        bpy.data.meshes.remove(old_mesh)
    records.append(dict(name=obj.name, body=body, font=FONT, size=size,
                        vertices=len(mesh.vertices), triangles=len(mesh.polygons)))

bpy.context.view_layer.update()
assert all(signature(scene.objects[name]) == digest for name, digest in unchanged.items())
scene['map_font'] = FONT
meshes = [obj for obj in scene.objects if obj.type == 'MESH']
stats = json.loads((OUT / 'build-report.json').read_text(encoding='utf-8'))
stats.update(objects=len(meshes), vertices=sum(len(o.data.vertices) for o in meshes),
             triangles=sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in meshes),
             map_font=FONT, text_objects=len(targets))
staged = ART / 'elf-city.blend'
bpy.ops.wm.save_as_mainfile(filepath=str(staged), relative_remap=False)
print('MAP_FONT_SCENE_SAVED', len(unchanged), 'other objects preserved', flush=True)

# Match the original spatial/material batching and Fyrox sRGB export.
groups = {}
for obj in meshes:
    center = sum((obj.matrix_world @ Vector(c) for c in obj.bound_box), Vector()) / 8
    group = (math.floor(center.x / 20), math.floor(-center.y / 20), obj.data.materials[0].name)
    groups.setdefault(group, []).append(obj)
exports = []
bpy.ops.object.select_all(action='DESELECT')
for group, objects in groups.items():
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    if len(objects) > 1:
        bpy.ops.object.join()
    obj = bpy.context.object
    obj.name = 'ElfCity_%s_%s_%s' % group
    exports.append(obj)
    obj.select_set(False)
    if len(exports) % 50 == 0:
        print('MAP_FONT_BATCH', len(exports), '/', len(groups), flush=True)
for material in bpy.data.materials:
    if 'srgb' in material:
        rgba = tuple(v / 255 for v in material['srgb']) + (1,)
        material.diffuse_color = rgba
        material.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value = rgba
bpy.ops.object.select_all(action='DESELECT')
for obj in exports:
    obj.select_set(True)
bpy.ops.export_scene.fbx(filepath=str(ART / 'elf-city.fbx'), use_selection=True,
                         object_types={'MESH'}, apply_unit_scale=True,
                         axis_forward='-Z', axis_up='Y', bake_anim=False, path_mode='AUTO')
stats['export_chunks'] = len(exports)
assert all(sha(OUT / name) == digest for name, digest in original_hashes.items()), 'Source changed; result staged only'
shutil.copy2(staged, OUT / 'elf-city.blend')
shutil.copy2(ART / 'elf-city.fbx', OUT / 'elf-city.fbx')
(OUT / 'build-report.json').write_text(json.dumps(stats, indent=2) + '\n', encoding='utf-8')
report = dict(font=FONT, font_sha256=sha(ROOT / FONT), text_objects=records,
              unchanged_objects_verified=len(unchanged), layout_unchanged=True,
              files={name: sha(OUT / name) for name in files})
(ART / 'verification.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print('MAP_FONT_UPDATE_OK', json.dumps(stats), flush=True)
