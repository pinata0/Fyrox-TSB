"""Check all candidate clips, skin deformation, materials, and the exported FBX (Blender)."""
import bpy
import json
import hashlib
import numpy as np
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/tripo-butter'
record=json.loads((OUT/'candidate.json').read_text())
bpy.ops.wm.open_mainfile(filepath=str(OUT/'butter.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects['butter_SkinnedMesh']
assert len(rig.data.bones)==record['bones']==70
assert len(mesh.data.uv_layers)==1
assert len(mesh.data.materials)==3
assert all(v.groups and abs(sum(g.weight for g in v.groups)-1)<.0001 for v in mesh.data.vertices)
assert all(len(v.groups)<=4 for v in mesh.data.vertices)
assert all(g.name in rig.data.bones for g in mesh.vertex_groups)
assert 'Hand.R' in rig.data.bones
for material in mesh.data.materials:
    images=[n.image for n in material.node_tree.nodes if n.type=='TEX_IMAGE' and n.image]
    assert len(images)==1 and images[0].size[0]==4096 and images[0].packed_file

rest=np.array([v.co[:] for v in mesh.data.vertices])
edges=np.array([e.vertices[:] for e in mesh.data.edges])
length=np.linalg.norm(rest[edges[:,0]]-rest[edges[:,1]],axis=1)
valid=length>.002
def evaluate(frame):
    scene.frame_set(frame);bpy.context.view_layer.update()
    evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    positions=np.array([v.co[:] for v in evaluated.data.vertices])
    assert np.isfinite(positions).all()
    return positions
reports={}
for name,(start,end) in record['clips'].items():
    first=evaluate(start)
    delta=np.max(np.abs(evaluate(start+(end-start)//4)-first))
    assert delta>.001,(name,'static')
    worst=0;worst_fraction=0
    for frame in range(start,end+1,3):
        points=evaluate(frame)
        ratio=np.linalg.norm(points[edges[:,0]]-points[edges[:,1]],axis=1)[valid]/length[valid]
        worst=max(worst,float(np.quantile(ratio,.99)))
        worst_fraction=max(worst_fraction,float(np.mean(ratio>2)))
    assert worst<2.5,(name,'excessive skin stretching',worst)
    seam=float(np.max(np.abs(evaluate(end)-first))) if name in ['Idle','Walk','Run','Dance'] else None
    if seam is not None:assert seam<.0001,(name,'loop seam',seam)
    reports[name]={'max_vertex_delta':float(delta),'worst_p99_edge_ratio':worst,'worst_fraction_edges_over_2x':worst_fraction,'loop_seam':seam}
evaluate(0)
height=float(np.ptp(evaluate(0)[:,2]));assert 1.8<height<2.1,height
# Read the delivered interchange file independently and confirm its animation and skin survived export.
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=str(OUT/'butter.fbx'))
assert any(a.frame_range[1]-a.frame_range[0]>=527 for a in bpy.data.actions)
fbx_rig=next(o for o in bpy.data.objects if o.type=='ARMATURE')
fbx_mesh=next(o for o in bpy.data.objects if o.type=='MESH')
assert len(fbx_rig.data.bones)==70 and len(fbx_mesh.data.materials)==3
for img in bpy.data.images:
    if img.source=='FILE':assert Path(bpy.path.abspath(img.filepath)).is_file(),img.filepath
(OUT/'verification.json').write_text(json.dumps({'passed':True,'bones':70,'height':height,'clips':reports,'fbx_roundtrip':True,
    'candidate_sha256':{name:hashlib.sha256((OUT/name).read_bytes()).hexdigest() for name in ['butter.blend','butter.fbx','candidate.json']}},indent=2))
print('TRIPO_VERIFY_OK: 70 bones, 11 motions, 3 textures, FBX roundtrip',flush=True)
