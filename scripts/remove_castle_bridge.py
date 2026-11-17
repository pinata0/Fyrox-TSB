"""Remove the B bridge from the saved scene while preserving other manual edits.

blender --background --python-exit-code 1 --python scripts/remove_castle_bridge.py
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
from castle_layout import make_layout

OUT = ROOT / 'assets/maps/fairy-kingdom-castle'
ART = ROOT / 'artifacts/castle-bridge-removal'
ART.mkdir(parents=True, exist_ok=True)
source = OUT / 'fairy-kingdom-castle.blend'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source_hash = sha(source)
layout_path = OUT / 'layout.json'
layout_hash = sha(layout_path)
backup = ART / ('source-' + source_hash[:12] + '.blend')
if not backup.exists():
    shutil.copy2(source, backup)
old = json.loads(layout_path.read_text(encoding='utf-8'))
layout = make_layout()
key = lambda b: json.dumps(b, sort_keys=True)
new_indices = {key(b): i for i, b in enumerate(layout['blocks'])}
old_keys = {key(b) for b in old['blocks']}
removed_blocks = {i: b for i, b in enumerate(old['blocks']) if key(b) not in new_indices}
added_blocks = [(i, b) for i, b in enumerate(layout['blocks']) if key(b) not in old_keys]
assert all('bridge' in b['name'].lower() or 'upper landing' in b['name']
           or 'upper door' in b['name'] for b in removed_blocks.values())
assert all(b['name'].endswith('closed upper doorway') for _, b in added_blocks)
bpy.ops.wm.open_mainfile(filepath=str(source))
bpy.context.preferences.filepaths.file_preview_type = 'NONE'
if bpy.context.object and bpy.context.object.mode != 'OBJECT':
    bpy.ops.object.mode_set(mode='OBJECT')
scene = bpy.context.scene


def signature(obj):
    data = dict(matrix=[list(row) for row in obj.matrix_world],
                collections=[c.name for c in obj.users_collection],
                hide_render=obj.hide_render, hide_viewport=obj.hide_viewport,
                properties={k: v for k, v in obj.items() if k != 'collision_index'})
    if obj.type == 'MESH':
        data.update(vertices=[list(v.co) for v in obj.data.vertices],
                    faces=[list(p.vertices) for p in obj.data.polygons],
                    uv=[[list(v.uv) for v in layer.data] for layer in obj.data.uv_layers],
                    materials=[m.name for m in obj.data.materials])
    return hashlib.sha256(json.dumps(data, sort_keys=True, default=str).encode()).hexdigest()


def loc(p):
    return Vector((p[0], -p[2], p[1]))


removed = {o for o in scene.objects if o.get('collision_index') in removed_blocks
           or 'B bridge' in o.name or 'B royal bridge' in o.name or 'B north upper landing' in o.name}
# Remove the old upper-door sill trim along with its wall segment.
for block in removed_blocks.values():
    p, s = block['center'], block['size']
    if s[1] > 2 and min(s[0], s[2]) < .6:
        for y in [.22, p[1] + s[1] / 2 - .2]:
            removed.update(o for o in scene.objects if o.name.startswith('Wall carved stringcourse')
                           and (o.location - loc((p[0], y, p[2]))).length < .001)
unchanged = {o: signature(o) for o in scene.objects if o not in removed}
removed_names = sorted(o.name for o in removed)
for o in removed:
    bpy.data.objects.remove(o, do_unlink=True)
for o in scene.objects:
    if 'collision_index' in o:
        previous = old['blocks'][o['collision_index']]
        i = new_indices[key(previous)]
        o['collision_index'] = i
        if o.name[:3].isdigit() and o.name[3:4] == ' ':
            o.name = '%03d %s' % (i, previous['name'])


def cube(name, center, size, material, bevel):
    mesh = bpy.data.meshes.new(name)
    vertices = [(x * size[0] / 2, y * size[2] / 2, z * size[1] / 2)
                for x, y, z in [(-1,-1,-1),(-1,-1,1),(-1,1,-1),(-1,1,1),
                                 (1,-1,-1),(1,-1,1),(1,1,-1),(1,1,1)]]
    mesh.from_pydata(vertices, [], [(0,4,6,2),(1,3,7,5),(0,1,5,4),(2,6,7,3),(0,2,3,1),(4,5,7,6)])
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.bevel(bm, geom=list(bm.edges), offset=min(bevel, min(size) * .2), segments=2, affect='EDGES')
    bmesh.ops.triangulate(bm, faces=list(bm.faces))
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    assert all(e.is_manifold for e in bm.edges)
    assert all(f.calc_area() > 1e-10 for f in bm.faces)
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()
    uv = mesh.uv_layers.new(name='Planar detail UV')
    for face in mesh.polygons:
        axis = max(range(3), key=lambda i: abs(face.normal[i]))
        axes = [i for i in range(3) if i != axis]
        for li in face.loop_indices:
            co = mesh.vertices[mesh.loops[li].vertex_index].co
            uv.data[li].uv = (co[axes[0]] * .25, co[axes[1]] * .25)
    obj = bpy.data.objects.new(name, mesh)
    obj.location = loc(center)
    bpy.data.collections['02 Shared structure and classical columns'].objects.link(obj)
    mesh.materials.append(material)
    return obj


for i, b in added_blocks:
    p, s = b['center'], b['size']
    obj = cube('%03d %s' % (i, b['name']), p, s, bpy.data.materials['Ivory limestone'], .035)
    obj['collision_index'] = i
    for y in [.22, p[1] + s[1] / 2 - .2]:
        cube('Wall carved stringcourse', (p[0], y, p[2]), (s[0] + .08, .15, s[2] + .08),
             bpy.data.materials['Painted cream wood'], .025)
bpy.context.view_layer.update()
assert all(signature(o) == digest for o, digest in unchanged.items())
assert not any('bridge' in o.name.lower() or 'B north upper landing' in o.name for o in scene.objects)
assert '052 Outer gate wall lintel' in scene.objects
staged = ART / source.name
bpy.ops.wm.save_as_mainfile(filepath=str(staged), relative_remap=False)
meshes = [o for o in scene.objects if o.type == 'MESH']
stats = dict(objects=len(meshes), vertices=sum(len(o.data.vertices) for o in meshes),
             triangles=sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in meshes),
             collision_blocks=len(layout['blocks']), materials=len(bpy.data.materials), slug=layout['slug'])
print('BRIDGE_REMOVED', len(removed_names), 'objects;', len(unchanged), 'other objects preserved', flush=True)

# The same regional/material batches and color conversion as the castle builder.
groups = {}
for o in meshes:
    center = sum((o.matrix_world @ Vector(c) for c in o.bound_box), Vector()) / 8
    k = (math.floor(center.x / 12), math.floor(-center.y / 16), o.data.materials[0].name if o.data.materials else '')
    groups.setdefault(k, []).append(o)
exports = []
for k, objects in groups.items():
    bpy.ops.object.select_all(action='DESELECT')
    for o in objects:
        o.hide_set(False)
        o.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    if len(objects) > 1:
        bpy.ops.object.join()
    o = bpy.context.object
    o.name = 'Castle chunk %s %s %s' % k
    exports.append(o)
for mat in bpy.data.materials:
    if 'srgb' in mat:
        rgba = tuple(v / 255 for v in mat['srgb']) + (1,)
        mat.diffuse_color = rgba
        mat.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value = rgba
bpy.ops.object.select_all(action='DESELECT')
for o in exports:
    o.select_set(True)
bpy.ops.export_scene.fbx(filepath=str(ART / 'fairy-kingdom-castle.fbx'), use_selection=True,
                         object_types={'MESH'}, apply_unit_scale=True, axis_forward='-Z', axis_up='Y',
                         bake_anim=False, path_mode='AUTO', use_triangles=True)
stats['export_chunks'] = len(exports)
assert sha(source) == source_hash and sha(layout_path) == layout_hash, 'Source changed during removal; result staged only'
shutil.copy2(staged, source)
shutil.copy2(ART / 'fairy-kingdom-castle.fbx', OUT / 'fairy-kingdom-castle.fbx')
layout_path.write_text(json.dumps(layout, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
(OUT / 'build-report.json').write_text(json.dumps(stats, indent=2) + '\n', encoding='utf-8')
report = dict(removed_objects=removed_names, removed_blocks=list(removed_blocks.values()),
              unchanged_objects_verified=len(unchanged), filled_upper_doorways=len(added_blocks), stats=stats,
              files={name: sha(OUT / name) for name in [source.name, 'fairy-kingdom-castle.fbx', 'layout.json']})
(ART / 'verification.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('CASTLE_BRIDGE_REMOVAL_OK', json.dumps(stats), flush=True)
