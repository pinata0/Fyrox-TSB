"""Inspect delivered remesh sources and production rest meshes with Blender."""
import bpy
import json
import sys
import numpy as np
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/lowpoly-import'
OUT.mkdir(parents=True, exist_ok=True)

def inspect(path):
    bpy.ops.wm.open_mainfile(filepath=str(path))
    bpy.context.scene.frame_set(0)
    for obj in bpy.data.objects:
        if obj.type == 'ARMATURE': obj.data.pose_position = 'REST'
    bpy.context.view_layer.update()
    meshes=[]
    arrays={}
    for obj in bpy.context.scene.objects:
        if obj.type != 'MESH': continue
        mesh=obj.data
        mesh.calc_loop_triangles()
        points=np.array([obj.matrix_world @ v.co for v in mesh.vertices])
        arrays[obj.name]=points
        meshes.append(dict(name=obj.name, vertices=len(points), triangles=len(mesh.loop_triangles),
            bounds=[points.min(0).tolist(),points.max(0).tolist()] if len(points) else [],
            matrix=[list(row) for row in obj.matrix_world],
            groups=[g.name for g in obj.vertex_groups],
            modifiers=[(m.name,m.type) for m in obj.modifiers],
            shapes=[k.name for k in mesh.shape_keys.key_blocks] if mesh.shape_keys else [],
            materials=[m.name if m else None for m in mesh.materials],uv_layers=list(mesh.uv_layers.keys())))
    np.savez_compressed(OUT/(path.stem+'.npz'),**arrays)
    return dict(file=path.relative_to(ROOT).as_posix(), meshes=meshes,
        images=[dict(name=i.name,size=list(i.size),packed=bool(i.packed_file),path=i.filepath) for i in bpy.data.images],
        bones={o.name:len(o.data.bones) for o in bpy.data.objects if o.type=='ARMATURE'},
        frames=[bpy.context.scene.frame_start,bpy.context.scene.frame_end])

paths=sorted((ROOT/'temp').glob('*.blend'))
if not paths: paths=sorted((ROOT/'assets/character-sources/lowpoly').rglob('*.blend'))
paths+=sorted((ROOT/'assets/characters').glob('*.blend'))
records=[]
for path in paths:
    record=inspect(path)
    records.append(record)
    print('INSPECT',path.name,[(m['name'],m['triangles']) for m in record['meshes']],flush=True)
    (OUT/'inventory.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
