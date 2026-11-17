"""Patch the saved castle without rebuilding or discarding manual scene edits.

blender --background --python-exit-code 1 --python scripts/repair_castle_gate.py
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
from castle_gate_arch import configure_outer_gate, lintel_geometry, moulding_points

OUT = ROOT / 'assets/maps/fairy-kingdom-castle'
ART = ROOT / 'artifacts/castle-gate-arch'
ART.mkdir(parents=True, exist_ok=True)
source = OUT / 'fairy-kingdom-castle.blend'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source_hash = sha(source)
layout_path = OUT / 'layout.json'
layout_hash = sha(layout_path)
backup = ART / ('source-' + source_hash[:12] + '.blend')
if not backup.exists():
    shutil.copy2(source, backup)
bpy.ops.wm.open_mainfile(filepath=str(source))
if hasattr(bpy.context.preferences.filepaths, 'file_preview_type'):
    bpy.context.preferences.filepaths.file_preview_type = 'NONE'
if bpy.context.object and bpy.context.object.mode != 'OBJECT':
    bpy.ops.object.mode_set(mode='OBJECT')
scene = bpy.context.scene
layout = json.loads(layout_path.read_text(encoding='utf-8'))
index, block = configure_outer_gate(layout)
gate = next(g for g in layout['gates'] if g['name'] == 'Outer crown gate')
targets = [o for o in scene.objects if o.get('collision_index') == index]
assert len(targets) == 1, 'Expected one outer gate lintel'
lintel = targets[0]
moulding = bpy.data.objects['Gate moulding']
assert (moulding.location - Vector((2, 8, 0))).length < .001
thresholds = [o for o in scene.objects if o.name.startswith('Wall carved stringcourse')
              and (o.location - Vector((2, 8, .22))).length < .001]
assert len(thresholds) <= 1
changed = {lintel, moulding, *thresholds}


def signature(o):
    data = dict(matrix=[list(row) for row in o.matrix_world],
                collections=[c.name for c in o.users_collection], hide_render=o.hide_render,
                hide_viewport=o.hide_viewport, properties=dict(o.items()))
    if o.type == 'MESH':
        data.update(vertices=[list(v.co) for v in o.data.vertices],
                    faces=[list(p.vertices) for p in o.data.polygons],
                    uv=[[list(v.uv) for v in layer.data] for layer in o.data.uv_layers],
                    materials=[m.name for m in o.data.materials])
    return hashlib.sha256(json.dumps(data, sort_keys=True, default=str).encode()).hexdigest()


unchanged = {o.name: signature(o) for o in scene.objects if o not in changed}


def loc(p):
    return Vector((p[0], -p[2], p[1]))


def replace_mesh(obj, vertices, faces):
    inverse = obj.matrix_world.inverted()
    mesh = bpy.data.meshes.new(obj.name + ' arched mesh')
    mesh.from_pydata([inverse @ loc(p) for p in vertices], [], faces)
    for mat in obj.data.materials:
        mesh.materials.append(mat)
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    assert all(e.is_manifold for e in bm.edges), obj.name
    assert bm.calc_volume(signed=True) > 0, obj.name
    assert all(f.calc_area() > 1e-10 for f in bm.faces), obj.name
    # Retain editable quads in the blend; FBX triangulates on export.
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
    old = obj.data
    obj.data = mesh
    if old.users == 0:
        bpy.data.meshes.remove(old)


replace_mesh(lintel, *lintel_geometry(block))
lintel.name = '%03d %s' % (index, block['name'])
lintel_name = lintel.name
lintel['arch_spring'] = gate['arch']['spring']
lintel['arch_rise'] = gate['arch']['rise']
points = [Vector((x + gate['x'], y, z + gate['z']))
          for x, y, z in moulding_points(gate['opening'], gate['arch'])]
vertices, faces = [], []
for i, p in enumerate(points):
    tangent = (points[min(i + 1, len(points) - 1)] - points[max(i - 1, 0)]).normalized()
    normal = Vector((-tangent.y, tangent.x, 0))
    for j in range(8):
        angle = j * math.tau / 8
        vertices.append(p + .1 * (normal * math.cos(angle) + Vector((0, 0, 1)) * math.sin(angle)))
for i in range(len(points) - 1):
    for j in range(8):
        a, b = i * 8 + j, i * 8 + (j + 1) % 8
        faces.append((a, b, b + 8, a + 8))
faces.extend([tuple(reversed(range(8))), tuple((len(points) - 1) * 8 + j for j in range(8))])
replace_mesh(moulding, vertices, faces)
for p in moulding.data.polygons:
    p.use_smooth = True
for o in thresholds:
    bpy.data.objects.remove(o, do_unlink=True)

# Rays prove that the opening is actually cut through the wall, not painted on it.
bpy.context.view_layer.update()
def wall_hit(x, height):
    inverse = lintel.matrix_world.inverted()
    return lintel.ray_cast(inverse @ Vector((x, 9, height)),
                          inverse.to_3x3() @ Vector((0, -1, 0)), distance=2)[0]

assert not wall_hit(2, 4.8)
assert wall_hit(2, 5.2)
assert wall_hit(-1.9, 3.7) and wall_hit(5.9, 3.7)
assert all(signature(bpy.data.objects[name]) == digest for name, digest in unchanged.items())
bpy.ops.object.select_all(action='DESELECT')
lintel.select_set(True)
bpy.context.view_layer.objects.active = lintel
staged = ART / source.name
bpy.ops.wm.save_as_mainfile(filepath=str(staged), relative_remap=False)
meshes = [o for o in scene.objects if o.type == 'MESH']
stats = dict(objects=len(meshes), vertices=sum(len(o.data.vertices) for o in meshes),
             triangles=sum(sum(len(p.vertices) - 2 for p in o.data.polygons) for o in meshes),
             collision_blocks=len(layout['blocks']), materials=len(bpy.data.materials), slug=layout['slug'])

# Same regional/material batching and Fyrox color conversion as build_castle.py.
groups = {}
for o in meshes:
    center = sum((o.matrix_world @ Vector(c) for c in o.bound_box), Vector()) / 8
    key = (math.floor(center.x / 12), math.floor(-center.y / 16), o.data.materials[0].name if o.data.materials else '')
    groups.setdefault(key, []).append(o)
exports = []
for key, objects in groups.items():
    bpy.ops.object.select_all(action='DESELECT')
    for o in objects:
        o.hide_set(False)
        o.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    if len(objects) > 1:
        bpy.ops.object.join()
    o = bpy.context.object
    o.name = 'Castle chunk %s %s %s' % key
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
assert sha(source) == source_hash, 'Source was edited during repair; staged result kept, source not overwritten'
assert sha(layout_path) == layout_hash, 'Layout was edited during repair; source not overwritten'
shutil.copy2(staged, source)
shutil.copy2(ART / 'fairy-kingdom-castle.fbx', OUT / 'fairy-kingdom-castle.fbx')
layout_path.write_text(json.dumps(layout, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
(OUT / 'build-report.json').write_text(json.dumps(stats, indent=2) + '\n', encoding='utf-8')
report = dict(source_backup=backup.name, unchanged_objects_verified=len(unchanged),
              modified_objects=[lintel_name, 'Gate moulding'], removed_thresholds=len(thresholds),
              geometry_checks=['closed manifold', 'positive volume', 'nondegenerate faces', 'arch ray casts'],
              stats=stats, files={name: sha(OUT / name) for name in [source.name, 'fairy-kingdom-castle.fbx', 'layout.json']})
(ART / 'verification.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('CASTLE_GATE_ARCH_OK', json.dumps(report), flush=True)
