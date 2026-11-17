"""Render the edited hall and verify bridge geometry is absent in Blend and FBX."""
import json
from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/maps/fairy-kingdom-castle'
ART = ROOT / 'artifacts/castle-bridge-removal'


def verify_geometry():
    bpy.context.view_layer.update()
    objects = [o for o in bpy.context.scene.objects if o.type == 'MESH']
    for x, z in [(30, 2), (40, 3.75), (40, 11.5), (40, 15), (40, 18.5), (40, 25.8)]:
        for obj in objects:
            inverse = obj.matrix_world.inverted()
            direction = inverse.to_3x3() @ Vector((0, 0, -1))
            hit = obj.ray_cast(inverse @ Vector((x, -z, 5)), direction.normalized(),
                               distance=4.8 * direction.length)[0]
            assert not hit, ('Bridge remnant', obj.name, x, z)
    return len(objects)


for label, path in [('before', ART / 'before/fairy-kingdom-castle.blend'),
                    ('after', OUT / 'fairy-kingdom-castle.blend')]:
    bpy.ops.wm.open_mainfile(filepath=str(path))
    scene = bpy.context.scene
    if label == 'after':
        assert not any('bridge' in o.name.lower() for o in scene.objects)
        assert '052 Outer gate wall lintel' in scene.objects
        assert len([o for o in scene.objects if 'closed upper doorway' in o.name]) == 2
        verify_geometry()
    bpy.ops.object.camera_add(location=(30.5, -15, 3.7))
    camera = bpy.context.object
    camera.rotation_euler = (Vector((46, -15, 3.5)) - camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera.data.lens = 22
    scene.camera = camera
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = 24
    scene.render.resolution_x = 1100
    scene.render.resolution_y = 760
    scene.render.resolution_percentage = 100
    scene.render.filepath = str(ART / (label + '.png'))
    bpy.ops.render.render(write_still=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=str(OUT / 'fairy-kingdom-castle.fbx'))
count = verify_geometry()
report_path = ART / 'verification.json'
report = json.loads(report_path.read_text(encoding='utf-8'))
report.update(saved_blend_verified=True, fbx_geometry_verified=True, fbx_meshes=count,
              renders=['before.png', 'after.png'])
report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('CASTLE_BRIDGE_REMOVAL_VERIFIED', count, 'FBX chunks', flush=True)
