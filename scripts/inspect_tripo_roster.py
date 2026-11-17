"""Inspect the uploaded roster archives in Blender without changing their data."""
import bpy
import json
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/tripo-roster-import'
report = {}
for directory in sorted((OUT / 'extracted').iterdir()):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(next(directory.glob('*.fbx'))))
    meshes = [o for o in bpy.context.scene.objects if o.type == 'MESH']
    rigs = [o for o in bpy.context.scene.objects if o.type == 'ARMATURE']
    texture = next(directory.rglob('*.jpg'))
    for img in bpy.data.images:
        if img.source == 'FILE':
            img.filepath = str(texture)
            img.reload()
    points = [o.matrix_world @ v.co for o in meshes for v in o.data.vertices]
    bb = [[min(p[i] for p in points), max(p[i] for p in points)] for i in range(3)]
    report[directory.name] = {
        'bounds': bb,
        'meshes': [{'name': o.name, 'vertices': len(o.data.vertices),
                    'shape_keys': list(o.data.shape_keys.key_blocks.keys()) if o.data.shape_keys else [],
                    'groups': [g.name for g in o.vertex_groups]} for o in meshes],
        'rigs': [{'bones': [{'name': b.name, 'head': list(o.matrix_world @ b.head_local),
                            'tail': list(o.matrix_world @ b.tail_local)} for b in o.data.bones]} for o in rigs],
        'actions': [a.name for a in bpy.data.actions]}
    center = Vector([(a+b)/2 for a,b in bb])
    size = max(b-a for a,b in bb)
    scene = bpy.context.scene
    scene.world = bpy.data.worlds.new('World')
    scene.world.color = (.65,.65,.65)
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = 8
    scene.render.resolution_x = scene.render.resolution_y = 384
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    for offset,power in [((2,-4,5),450),((-3,-1,3),280),((0,3,4),450)]:
        bpy.ops.object.light_add(type='AREA', location=center+Vector(offset)*size)
        lamp=bpy.context.object
        lamp.data.energy=power*size*size
        lamp.data.size=2*size
        lamp.rotation_euler=(center-lamp.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=center+Vector((0,-3,0))*size)
    camera=bpy.context.object
    camera.data.type='ORTHO'
    camera.data.ortho_scale=size*1.2
    camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler()
    scene.camera=camera
    scene.render.filepath=str(OUT/(directory.name+'.png'))
    bpy.ops.render.render(write_still=True)
    (OUT/'source-inspection.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print('SOURCE_INSPECTED',directory.name,flush=True)
