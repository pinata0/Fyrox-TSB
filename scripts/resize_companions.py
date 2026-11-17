"""Resize Sebastian and Eru against the saved original-game references.

Blender --background --factory-startup --python-exit-code 1 --python
scripts/resize_companions.py [-- --skip-render | --publish]
Candidates always start from the once-only backup, so reruns cannot compound scale.
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
from export_fyrox_fbx import export_fbx, resolve_character_textures

WORK = ROOT / 'artifacts/companion-size'
BEFORE = WORK / 'before'
OUT = WORK / 'candidate'
ASSETS = ROOT / 'assets/characters'
SPECS = {
    'chloe': dict(mesh='Sebastian', scale=2.40, old_base=(0, .40, .67),
                  new_base=(0, .58, .03), rider_lift=0.,
                  reference='artifacts/tripo-sebastian-original-style/references/chloe-official-jp-basic.jpg'),
    'ui': dict(mesh='Frog', scale=1.55, old_base=(0, -.04, 0),
               new_base=(0, -.04, .025), rider_lift=.30,
               reference='assets/references/ui-fullbody.png'),
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def coords(mesh):
    return np.array([v.co[:] for v in mesh.data.vertices])


def render(scene, slug, label, views=('front', 'side')):
    for obj in scene.objects:
        if '_RefinedViewSleeve.' in obj.name:
            obj.hide_render = True
        if obj.type in {'LIGHT', 'CAMERA'}:
            bpy.data.objects.remove(obj, do_unlink=True)
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = 16
    scene.render.resolution_x = scene.render.resolution_y = 640
    scene.render.resolution_percentage = 100
    scene.view_settings.view_transform = 'Standard'
    scene.world = bpy.data.worlds.new('CompanionReviewWorld')
    scene.world.use_nodes = True
    scene.world.node_tree.nodes['Background'].inputs[0].default_value = (.19, .21, .25, 1)
    scene.world.node_tree.nodes['Background'].inputs[1].default_value = .8
    for loc, energy, size in [((3, -4, 5), 400, 5), ((-3, -2, 3), 200, 4), ((0, 3, 4), 250, 3)]:
        bpy.ops.object.light_add(type='AREA', location=loc)
        lamp = bpy.context.object
        lamp.data.energy = energy
        lamp.data.size = size
        lamp.rotation_euler = (Vector((0, 0, 1.3)) - lamp.location).to_track_quat('-Z', 'Y').to_euler()
    bpy.ops.object.camera_add()
    scene.camera = bpy.context.object
    scene.camera.data.type = 'ORTHO'
    scene.camera.data.ortho_scale = 3.65 if slug == 'chloe' else 3.15
    for view in views:
        scene.camera.location = {'front': (0, -6, 1.6), 'side': (6, 0, 1.6),
                                 'three-quarter': (3, -6, 2.)}[view]
        scene.camera.rotation_euler = (Vector((0, .15, 1.50 if slug == 'chloe' else 1.33)) - scene.camera.location).to_track_quat('-Z', 'Y').to_euler()
        scene.render.filepath = str(WORK / f'{slug}-{label}-{view}.png')
        bpy.ops.render.render(write_still=True)


def build(slug, spec):
    names = [f'{slug}.blend', f'{slug}.fbx', f'{slug}.morphs.json']
    previous = json.loads((WORK / 'verification.json').read_text()) if (WORK / 'verification.json').exists() else {}
    replaces = {name: digest(ASSETS / name) for name in names
                if previous.get('sha256', {}).get(name) == digest(ASSETS / name)}
    for name in names:
        if not (BEFORE / name).exists():
            shutil.copy2(ASSETS / name, BEFORE / name)
    bpy.ops.wm.open_mainfile(filepath=str(BEFORE / f'{slug}.blend'))
    scene = bpy.context.scene
    scene.frame_set(0)
    rig = bpy.data.objects['TSB_Rig']
    meshes = [o for o in scene.objects if o.type == 'MESH']
    resolve_character_textures(meshes)
    for obj in meshes:
        for mat in obj.data.materials:
            if not mat or not mat.use_nodes:
                continue
            for node in mat.node_tree.nodes:
                if node.type != 'TEX_IMAGE' or not node.image:
                    continue
                image = node.image
                source = Path(bpy.path.abspath(image.filepath))
                assert source.is_file(), source
                if not (OUT / source.name).exists():
                    shutil.copy2(source, OUT / source.name)
                image.filepath = str(OUT / source.name)
    if '--skip-render' not in sys.argv:
        render(scene, slug, 'before', ('front',))
    mesh = bpy.data.objects[spec['mesh']]
    original = coords(mesh)
    untouched = {obj.name: coords(obj) for obj in meshes if obj != mesh}
    old, new = Vector(spec['old_base']), Vector(spec['new_base'])
    scale = spec['scale']
    for vertex in mesh.data.vertices:
        vertex.co = (vertex.co - old) * scale + new
    mesh.data.update()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode='EDIT')
    for bone in rig.data.edit_bones:
        if bone.name.startswith(spec['mesh']):
            bone.head = (bone.head - old) * scale + new
            bone.tail = (bone.tail - old) * scale + new
    bpy.ops.object.mode_set(mode='OBJECT')
    # Move the rider in the animation, keeping rest meshes, facial landmarks,
    # morphs and grip calibration intact. The offset follows Root during motion.
    lift = rig.data.bones['Hips'].matrix_local.to_3x3().inverted() @ Vector((0, 0, spec['rider_lift']))
    for frame in range(scene.frame_start, scene.frame_end + 1):
        scene.frame_set(frame)
        for bone in rig.pose.bones:
            if bone.name.startswith(spec['mesh']):
                bone.location *= scale
                bone.keyframe_insert(data_path='location', frame=frame, group=bone.name)
        if spec['rider_lift']:
            hips = rig.pose.bones['Hips']
            hips.location += lift
            hips.keyframe_insert(data_path='location', frame=frame, group=hips.name)
    # Longer arms and feet must not sink into the floor during the old clips.
    # Raise the companion only where necessary; Ui follows her mount's correction.
    grounding = []
    for frame in range(scene.frame_start, scene.frame_end + 1):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        points = np.empty(len(evaluated.data.vertices) * 3, dtype=np.float32)
        evaluated.data.vertices.foreach_get('co', points)
        dz = max(0., .008 - float(points.reshape(-1, 3)[:, 2].min()))
        grounding.append(dz)
        affected = [spec['mesh'] + 'Root'] + (['Hips'] if spec['rider_lift'] else [])
        for name in affected:
            bone = rig.pose.bones[name]
            parent = bone.parent
            basis = parent.matrix @ parent.bone.matrix_local.inverted() @ bone.bone.matrix_local
            bone.location += basis.to_3x3().inverted() @ Vector((0, 0, dz))
            bone.keyframe_insert(data_path='location', frame=frame, group=name)
    assert all(np.array_equal(points, coords(bpy.data.objects[name])) for name, points in untouched.items())
    report = dict(spec, old_dimensions_m=np.ptp(original, axis=0).tolist(),
                  new_dimensions_m=np.ptp(coords(mesh), axis=0).tolist(),
                  source_sha256={name: digest(BEFORE / name) for name in names},
                  replaces_sha256=replaces, grounding_lift_m=grounding,
                  clips=json.loads(scene['clips']))
    scene['companion_resize'] = json.dumps(report)
    scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.data.use_autopack = False
    for image in bpy.data.images:
        if image.source == 'FILE' and image.filepath and Path(bpy.path.abspath(image.filepath)).is_file():
            image.filepath = '//' + Path(bpy.path.abspath(image.filepath)).name
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT / f'{slug}.blend'), relative_remap=False)
    export_fbx(bpy.data.objects[f'{slug}_SkinnedMesh'], rig, OUT / f'{slug}.fbx',
               [obj for obj in meshes if obj.name != f'{slug}_SkinnedMesh'])
    if '--skip-render' not in sys.argv:
        render(scene, slug, 'after', ('front', 'side', 'three-quarter'))
    print('COMPANION_RESIZED_OK', slug, flush=True)
    return report


def main():
    BEFORE.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    if '--publish' in sys.argv:
        verification = json.loads((WORK / 'verification.json').read_text())
        reports = json.loads((WORK / 'resize.json').read_text())
        assert verification['passed']
        for slug, report in reports.items():
            for name, baseline in report['source_sha256'].items():
                expected = verification['sha256'][name]
                assert digest(OUT / name) == expected, name
                assert digest(ASSETS / name) in [baseline, expected, report.get('replaces_sha256', {}).get(name)], f'{name} changed independently'
        for name in verification['sha256']:
            shutil.copy2(OUT / name, ASSETS / name)
        records = json.loads((ASSETS / 'manifest.json').read_text(encoding='utf-8'))
        for row in records:
            if row['slug'] not in reports:
                continue
            row['companion_resize'] = reports[row['slug']]
            for key in ['blend', 'model']:
                row['sha256'][key] = digest(ROOT / row[key])
        (ASSETS / 'manifest.json').write_text(json.dumps(records, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        print('COMPANION_RESIZE_PUBLISHED', flush=True)
        return
    selected = sys.argv[sys.argv.index('--only') + 1] if '--only' in sys.argv else None
    reports = json.loads((WORK / 'resize.json').read_text()) if selected and (WORK / 'resize.json').exists() else {}
    reports.update({slug: build(slug, spec) for slug, spec in SPECS.items() if not selected or slug == selected})
    (WORK / 'resize.json').write_text(json.dumps(reports, indent=2))


if __name__ == '__main__':
    main()
