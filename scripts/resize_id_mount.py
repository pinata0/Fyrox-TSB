"""Enlarge Id's crescent around its floating base without changing the rider.

Run with Blender --background --python-exit-code 1 --python scripts/resize_id_mount.py.
The original assets are backed up once; repeated runs always use that baseline.
Inspect artifacts/id-mount-size/after-*.png and run the saved verification script,
then use --publish to apply the checked candidate to the runtime assets.
"""
import bpy
import hashlib
import json
import shutil
import sys
from pathlib import Path

import numpy as np
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from export_fyrox_fbx import export_fbx

WORK = ROOT / 'artifacts/id-mount-size'
ASSETS = ROOT / 'assets/characters'
BEFORE = WORK / 'before'
OUT = WORK / 'candidate'
SCALE = 1.7
OLD_BASE = Vector((0, .16, .05))
NEW_BASE = Vector((0, .45, .02))
ROBOT_OFFSET = Vector((.32, 0, .16))


def coords(mesh):
    return np.array([v.co[:] for v in mesh.data.vertices])


def render_views(scene):
    for obj in scene.objects:
        if '_RefinedViewSleeve.' in obj.name:
            obj.hide_render = True
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = 24
    scene.render.resolution_x = scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.view_settings.view_transform = 'Standard'
    scene.world = bpy.data.worlds.new('MountReviewWorld')
    scene.world.use_nodes = True
    scene.world.node_tree.nodes['Background'].inputs[0].default_value = (.24, .26, .30, 1)
    scene.world.node_tree.nodes['Background'].inputs[1].default_value = .8
    for loc, energy, size in [((3, -4, 5), 450, 5), ((-3, -2, 3), 250, 4), ((0, 3, 4), 300, 3)]:
        bpy.ops.object.light_add(type='AREA', location=loc)
        lamp = bpy.context.object
        lamp.data.energy = energy
        lamp.data.size = size
        lamp.rotation_euler = (Vector((0, 0, 1)) - lamp.location).to_track_quat('-Z', 'Y').to_euler()
    bpy.ops.object.camera_add()
    scene.camera = bpy.context.object
    scene.camera.data.type = 'ORTHO'
    scene.camera.data.ortho_scale = 2.85
    for label, location in [('front', (0, -5, 1.55)), ('side', (5, 0, 1.55)), ('three-quarter', (3, -5, 2.1)), ('back', (0, 5, 1.55))]:
        scene.camera.location = location
        scene.camera.rotation_euler = (Vector((0, .15, 1.1)) - scene.camera.location).to_track_quat('-Z', 'Y').to_euler()
        scene.render.filepath = str(WORK / f'after-{label}.png')
        bpy.ops.render.render(write_still=True)


def main():
    BEFORE.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    if '--publish' in sys.argv:
        verification = json.loads((WORK / 'asset-verification.json').read_text())
        assert verification['passed']
        for name, expected in verification['candidate_sha256'].items():
            assert hashlib.sha256((OUT / name).read_bytes()).hexdigest() == expected, name
        # Refuse to replace an independently edited character with our baseline.
        for name in ['id.blend', 'id.fbx']:
            current = hashlib.sha256((ASSETS / name).read_bytes()).hexdigest()
            baseline = hashlib.sha256((BEFORE / name).read_bytes()).hexdigest()
            assert current in [baseline, verification['candidate_sha256'][name]], name
        assert json.loads((OUT / 'id.morphs.json').read_text()) == json.loads((ASSETS / 'id.morphs.json').read_text())
        for name in ['id.blend', 'id.fbx']:
            shutil.copy2(OUT / name, ASSETS / name)
        records = json.loads((ASSETS / 'manifest.json').read_text(encoding='utf-8'))
        row = next(r for r in records if r['slug'] == 'id')
        for key in ['blend', 'model']:
            row['sha256'][key] = hashlib.sha256((ROOT / row[key]).read_bytes()).hexdigest()
        row['mount_resize'] = json.loads((WORK / 'resize.json').read_text())
        (ASSETS / 'manifest.json').write_text(json.dumps(records, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        print('ID_MOUNT_PUBLISHED_OK', flush=True)
        return
    for name in ['id.blend', 'id.fbx', 'id.morphs.json', 'manifest.json']:
        if not (BEFORE / name).exists():
            shutil.copy2(ASSETS / name, BEFORE / name)
    bpy.ops.wm.open_mainfile(filepath=str(BEFORE / 'id.blend'))
    scene = bpy.context.scene
    scene.frame_set(0)
    rig = bpy.data.objects['TSB_Rig']
    meshes = [o for o in scene.objects if o.type == 'MESH']
    untouched = {m.name: coords(m) for m in meshes if m.name not in ['Crescent', 'Nata']}
    # Resolve the relative texture links against the production asset directory.
    for image in bpy.data.images:
        if image.source == 'FILE':
            path = ASSETS / Path(bpy.path.abspath(image.filepath)).name
            if path.is_file():
                local = OUT / path.name
                if not local.exists():
                    shutil.copy2(path, local)
                image.filepath = '//' + path.name
            else:
                assert image.packed_file or Path(bpy.path.abspath(image.filepath)).is_file(), image.filepath
    mount = bpy.data.objects['Crescent']
    original = coords(mount)
    for vertex in mount.data.vertices:
        relative = vertex.co - OLD_BASE
        # Keep the front lip at lap height while enlarging the backrest and
        # sides. Uniform scaling alone hides the rider's chest behind the rim.
        blend = max(0., min(1., (relative.y + .16) / .38))
        blend = blend * blend * (3 - 2 * blend)
        crown = max(0., min(1., (relative.z - .70) / .30))
        blend = max(blend, crown * crown * (3 - 2 * crown))
        vertex.co = relative * SCALE + NEW_BASE
        vertex.co.z = relative.z * (1.10 + (SCALE - 1.10) * blend) + NEW_BASE.z
    mount.data.update()
    for vertex in bpy.data.objects['Nata'].data.vertices:
        vertex.co += ROBOT_OFFSET
    bpy.data.objects['Nata'].data.update()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode='EDIT')
    for name, offset in [('CrescentRoot', NEW_BASE - OLD_BASE), ('NataRoot', ROBOT_OFFSET)]:
        bone = rig.data.edit_bones[name]
        bone.head += offset
        bone.tail += offset
    bpy.ops.object.mode_set(mode='OBJECT')
    assert all(np.array_equal(points, coords(bpy.data.objects[name])) for name, points in untouched.items())
    report = {
        'scale': SCALE,
        'old_dimensions_m': np.ptp(original, axis=0).tolist(),
        'new_dimensions_m': np.ptp(coords(mount), axis=0).tolist(),
        'base_m': list(NEW_BASE),
        'front_height_scale': 1.10,
        'robot_offset_m': list(ROBOT_OFFSET),
        'unchanged_rider_meshes': list(untouched),
        'clips': json.loads(scene['clips']),
    }
    scene['id_mount_resize'] = json.dumps(report)
    scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.data.use_autopack = False
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT / 'id.blend'), relative_remap=False)
    export_fbx(bpy.data.objects['id_SkinnedMesh'], rig, OUT / 'id.fbx',
               [m for m in meshes if m.name != 'id_SkinnedMesh'])
    report['candidate_sha256'] = {name: hashlib.sha256((OUT / name).read_bytes()).hexdigest()
                                 for name in ['id.blend', 'id.fbx']}
    (WORK / 'resize.json').write_text(json.dumps(report, indent=2))
    if '--skip-render' not in sys.argv:
        render_views(scene)
    print('ID_MOUNT_RESIZED_OK', flush=True)


if __name__ == '__main__':
    main()
