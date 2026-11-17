"""Normalize engine skin weights and correct foot penetration in staged motion."""
import bpy
import json
import sys
import numpy as np
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from import_meshy_characters import STAGE,OUT,SLUGS,setup_scene
from export_fyrox_fbx import export_fbx
reports=[]
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS
for slug in selected:
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects[f'{slug}_SkinnedMesh']
    meshes=[o for o in bpy.context.scene.objects if o.type=='MESH']
    for mesh in meshes:
        for v in mesh.data.vertices:
            weights=sorted([(g.group,g.weight) for g in v.groups if g.weight>1e-6],key=lambda a:a[1],reverse=True)[:4]
            total=sum(w for _,w in weights);assert total>0
            for group in list(v.groups):mesh.vertex_groups[group.group].remove([v.index])
            for group,w in weights:mesh.vertex_groups[group].add([v.index],w/total,'REPLACE')
    corrections={};root=rig.pose.bones['Root'];inverse=root.bone.matrix_local.to_3x3().inverted()
    for name,(start,end) in json.loads(scene['clips']).items():
        # Preserve the original death timing and ground-facing final silhouette.
        if name=='Death':continue
        samples=[]
        for f in range(start,end+1):
            scene.frame_set(f);bpy.context.view_layer.update()
            m=body.evaluated_get(bpy.context.evaluated_depsgraph_get());points=np.empty(len(m.data.vertices)*3,dtype=np.float32);m.data.vertices.foreach_get('co',points)
            floor=points.reshape(-1,3)[:,2].min()
            correction=max(0,float(-floor+.004))
            samples.append(correction)
        # Smooth a 5-frame window, preserving loop endpoints and sufficient clearance.
        smoothed=np.convolve(np.pad(samples,(2,2),mode='edge'),np.ones(5)/5,mode='valid')
        smoothed=np.maximum(samples,smoothed)
        if name in ['Idle','Walk','Run','Dance']:smoothed[-1]=smoothed[0]=max(smoothed[0],smoothed[-1])
        for f,amount in zip(range(start,end+1),smoothed):
            scene.frame_set(f);root.location=inverse@Vector((0,0,float(amount)));root.keyframe_insert(data_path='location',frame=f,group='Root')
        corrections[name]=float(max(smoothed))
    scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    export_fbx(body,rig,STAGE/f'{slug}.fbx',[m for m in meshes if m!=body])
    for frame,label in [(165,'attack'),(315,'dance')]:setup_scene(body,OUT/f'{slug}-{label}.png',frame)
    reports.append(dict(slug=slug,ground_corrections=corrections))
    (OUT/'motion-polish.json').write_text(json.dumps(reports,indent=2))
    print('MESHY_MOTION_OK',slug,flush=True)
