"""Reduce the sprite-derived rifle elbow snap for Shion's narrow Meshy sleeves."""
import bpy,sys,json
import numpy as np
from pathlib import Path
from mathutils import Quaternion
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from import_meshy_characters import STAGE,OUT,setup_scene
from export_fyrox_fbx import export_fbx
bpy.ops.wm.open_mainfile(filepath=str(STAGE/'shion.blend'))
rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects['shion_SkinnedMesh'];scene=bpy.context.scene
already=bool(scene.get('meshy_rifle_motion_refinement'))
for f in range(156,283):
    if 181<=f<222 or (already and f<=180):continue
    scene.frame_set(f)
    for p in rig.pose.bones:
        if p.name.startswith(('UpperArm','Forearm','Hand')):
            p.rotation_quaternion=Quaternion().slerp(p.rotation_quaternion,.40 if f<=180 else .80)
            p.keyframe_insert(data_path='rotation_quaternion',frame=f,group=p.name)
if not scene.get('meshy_sleeve_weights_smoothed'):
    w=np.zeros((len(mesh.data.vertices),len(mesh.vertex_groups)))
    for v in mesh.data.vertices:
        for g in v.groups:w[v.index,g.group]=g.weight
    edges=np.array([e.vertices[:] for e in mesh.data.edges]);counts=np.zeros(len(w))
    np.add.at(counts,edges[:,0],1);np.add.at(counts,edges[:,1],1)
    for _ in range(6):
        total=np.zeros_like(w);np.add.at(total,edges[:,0],w[edges[:,1]]);np.add.at(total,edges[:,1],w[edges[:,0]])
        w=w*.55+total/np.maximum(counts[:,None],1)*.45
    for v in mesh.data.vertices:
        values=w[v.index];keep=values.argsort()[-4:];values[[i for i in range(len(values)) if i not in keep]]=0;values/=values.sum()
        for g in list(v.groups):mesh.vertex_groups[g.group].remove([v.index])
        for i in keep:
            if values[i]>1e-6:mesh.vertex_groups[int(i)].add([v.index],float(values[i]),'REPLACE')
    scene['meshy_sleeve_weights_smoothed']=True
scene.frame_set(0);scene['meshy_rifle_motion_refinement']='40 percent attack and 80 percent ultimate angular amplitude; source timing preserved'
bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/'shion.blend'));export_fbx(mesh,rig,STAGE/'shion.fbx')
setup_scene(mesh,OUT/'shion-attack.png',165)
print('MESHY_SHION_REFINED',flush=True)
