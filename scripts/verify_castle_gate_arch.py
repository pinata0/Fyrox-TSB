"""Verify the saved Blender arch and exported FBX; render both sides for review."""
import json
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/maps/fairy-kingdom-castle'
ART = ROOT / 'artifacts/castle-gate-arch'
bpy.ops.wm.open_mainfile(filepath=str(OUT / 'fairy-kingdom-castle.blend'))
scene = bpy.context.scene
wall = bpy.data.objects['052 Outer gate wall lintel']
bm = bmesh.new()
bm.from_mesh(wall.data)
assert all(e.is_manifold for e in bm.edges)
assert all(f.calc_area() > 1e-10 for f in bm.faces)
assert bm.calc_volume(signed=True) > 0
bm.free()


def ray_checks(objects):
    def hit(x, height):
        origin, direction = Vector((x, 9, height)), Vector((0, -1, 0))
        for obj in objects:
            inverse = obj.matrix_world.inverted()
            local_direction = inverse.to_3x3() @ direction
            result = obj.ray_cast(inverse @ origin, local_direction.normalized(),
                                  distance=2 * local_direction.length)
            if result[0]:
                return True
        return False
    assert not hit(2, 4.7), 'Arch opening is filled'
    assert hit(2, 5.2), 'Missing arch crown'
    assert hit(-1.9, 3.7) and hit(5.9, 3.7), 'Missing arch shoulders'


ray_checks([wall])
scene.render.engine = 'CYCLES'
scene.cycles.samples = 24
scene.render.resolution_x = 1000
scene.render.resolution_y = 760
scene.render.resolution_percentage = 100
for filename, position, target in [
    ('after', (2, 24, 5.6), (2, 8, 3.5)),
    ('after-rear', (3, -3, 5.4), (2, 8, 3.6)),
]:
    bpy.ops.object.camera_add(location=position)
    camera = bpy.context.object
    camera.rotation_euler = (Vector(target) - camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera.data.type = 'ORTHO'
    camera.data.ortho_scale = 15
    scene.camera = camera
    scene.render.filepath = str(ART / (filename + '.png'))
    bpy.ops.render.render(write_still=True)

# Import the actual delivery, then check its geometry at the gate again.
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=str(OUT / 'fairy-kingdom-castle.fbx'))
bpy.context.view_layer.update()
meshes = [o for o in bpy.context.scene.objects if o.type == 'MESH']
ray_checks(meshes)
assert all(all(p.area > 1e-10 for p in o.data.polygons) for o in meshes)
report = json.loads((ART / 'verification.json').read_text(encoding='utf-8'))
report['saved_blend_verified'] = True
report['fbx_import_verified'] = True
report['fbx_meshes'] = len(meshes)
report['renders'] = ['after.png', 'after-rear.png']
(ART / 'verification.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('CASTLE_GATE_ARCH_DELIVERY_OK', len(meshes), 'FBX meshes', flush=True)
