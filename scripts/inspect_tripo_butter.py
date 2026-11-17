"""Inspect the uploaded Tripo FBXs without changing the originals (Blender)."""
import bpy
import json
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'assets/character-sources/tripo/butter'
OUT = ROOT / 'artifacts/tripo-butter'
OUT.mkdir(parents=True, exist_ok=True)

def bounds(obj):
    points = [obj.matrix_world @ v.co for v in obj.data.vertices]
    return [[min(p[i] for p in points), max(p[i] for p in points)] for i in range(3)]

report = {}
for kind in ['rigged', 'clothes', 'accessories']:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    fbx = next((SOURCE / kind).glob('*.fbx'))
    bpy.ops.import_scene.fbx(filepath=str(fbx))
    textures = list((SOURCE / kind).rglob('*.jpg'))
    for img in bpy.data.images:
        if img.source == 'FILE' and textures:
            img.filepath = str(textures[0]); img.reload()
    objects = []
    for obj in bpy.context.scene.objects:
        item = {'name': obj.name, 'type': obj.type, 'transform': [list(row) for row in obj.matrix_world]}
        if obj.type == 'MESH':
            item.update(vertices=len(obj.data.vertices), polygons=len(obj.data.polygons), bounds=bounds(obj),
                        groups=[g.name for g in obj.vertex_groups],
                        materials=[m.name if m else None for m in obj.data.materials])
        elif obj.type == 'ARMATURE':
            item['bones'] = [{'name': b.name, 'parent': b.parent.name if b.parent else None,
                              'head': list(obj.matrix_world @ b.head_local),
                              'tail': list(obj.matrix_world @ b.tail_local)} for b in obj.data.bones]
        objects.append(item)
    report[kind] = {'file': str(fbx), 'objects': objects,
                    'actions': [{'name': a.name, 'frames': list(a.frame_range)} for a in bpy.data.actions],
                    'materials': [{'name': m.name, 'nodes': [n.type for n in m.node_tree.nodes]} for m in bpy.data.materials if m.use_nodes]}
    points = [obj.matrix_world @ v.co for obj in bpy.context.scene.objects if obj.type == 'MESH' for v in obj.data.vertices]
    center = Vector(tuple((min(p[i] for p in points)+max(p[i] for p in points))/2 for i in range(3)))
    size = max(max(p[i] for p in points)-min(p[i] for p in points) for i in range(3))
    scene = bpy.context.scene
    scene.world = bpy.data.worlds.new('World'); scene.world.color = (.6,.6,.6)
    scene.render.engine = 'CYCLES'; scene.cycles.samples = 12
    scene.render.resolution_x = 640; scene.render.resolution_y = 640; scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = 'PNG'; scene.render.film_transparent = True
    for offset, power in [((2,-4,5),450),((-3,-1,3),280),((0,3,4),450)]:
        bpy.ops.object.light_add(type='AREA', location=center+Vector(offset)*size)
        lamp=bpy.context.object; lamp.data.energy=power*size*size; lamp.data.size=2*size
        lamp.rotation_euler=(center-lamp.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=center+Vector((0,-3,0))*size)
    cam=bpy.context.object;cam.data.type='ORTHO';cam.data.ortho_scale=size*1.2;scene.camera=cam
    cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler()
    scene.render.filepath=str(OUT/f'{kind}-source.png')
    bpy.ops.render.render(write_still=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/f'{kind}-source.blend'))
(OUT/'source-inspection.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
print('TRIPO_INSPECTION_OK',flush=True)
