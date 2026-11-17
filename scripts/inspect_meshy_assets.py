"""Inventory imported Meshy rigs and render source contact frames (Blender)."""
import bpy
import json
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/meshy-import/inspection'
OUT.mkdir(parents=True, exist_ok=True)
records = []
for i, path in enumerate(sorted((ROOT / 'assets/temp').glob('*.glb'))):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(path))
    meshes = [o for o in bpy.data.objects if o.type == 'MESH']
    rigs = [o for o in bpy.data.objects if o.type == 'ARMATURE']
    points = [o.matrix_world @ v.co for o in meshes for v in o.data.vertices]
    lo = Vector([min(p[a] for p in points) for a in range(3)])
    hi = Vector([max(p[a] for p in points) for a in range(3)])
    center = (lo + hi) / 2
    record = dict(index=i, file=path.name, bounds=[list(lo),list(hi)],
                  vertices=sum(len(o.data.vertices) for o in meshes),
                  bones=[dict(name=b.name,parent=b.parent.name if b.parent else None,
                              head=list(r.matrix_world @ b.head_local),
                              tail=list(r.matrix_world @ b.tail_local)) for r in rigs for b in r.data.bones])
    records.append(record)
    (OUT / 'inventory.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
    scene=bpy.context.scene
    scene.render.engine='CYCLES';scene.cycles.samples=8
    scene.render.resolution_x=440;scene.render.resolution_y=520;scene.render.resolution_percentage=100
    scene.world=bpy.data.worlds.new('ReviewWorld');scene.world.use_nodes=True
    scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.35,.35,.35,1)
    scene.world.node_tree.nodes['Background'].inputs[1].default_value=.8
    scene.view_settings.view_transform='Standard'
    for loc,energy,size in [((3,-4,5),500,5),((-3,-2,3),250,4),((0,3,4),300,3)]:
        bpy.ops.object.light_add(type='AREA',location=loc)
        bpy.context.object.data.energy=energy;bpy.context.object.data.shape='DISK';bpy.context.object.data.size=size
        bpy.context.object.rotation_euler=(center-bpy.context.object.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=center+Vector((0,-5,0)))
    camera=bpy.context.object;camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.type='ORTHO';camera.data.ortho_scale=max((hi-lo).z,(hi-lo).x*520/440)*1.16
    scene.camera=camera;scene.render.filepath=str(OUT/f'{i:02d}.png')
    bpy.ops.render.render(write_still=True)
    print('MESHY_INSPECT_OK',i,path.name,flush=True)
