"""Check feather isolation, attachment continuity and all twelve clips in Blender."""
import bpy,json,math
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'artifacts/erpin-motion'
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/erpin.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects['erpin_SkinnedMesh']
wing_groups={g.index for g in mesh.vertex_groups if g.name.startswith('Wing')}
weighted=[v.index for v in mesh.data.vertices if any(g.group in wing_groups and g.weight>1e-6 for g in v.groups)]
assert len(weighted)>600
for i in weighted:
    assert abs(sum(g.weight for g in mesh.data.vertices[i].groups)-1)<1e-5
    assert len(mesh.data.vertices[i].groups)<=4
assert all(rig.data.bones['Wing.'+s].parent.name=='Head' for s in ['L','R'])
assert all(rig.data.bones['WingTip.'+s].parent.name=='Wing.'+s for s in ['L','R'])
rest={s:rig.data.bones['Wing.'+s].head_local-rig.data.bones['Head'].head_local for s in ['L','R']}
records=[]
clips=json.loads((OUT/'bake-verification.json').read_text())['clip_ranges']
names=['Idle','Walk','Run','Jump','Attack1_1','Skill','Ultimate','Dance','Hurt','Death','Spawn','Attack2_1']
for name,(start,end,loop) in zip(names,clips):
    angles=[];attachment_error=0;points=[]
    for f in np.linspace(start,end,9):
        scene.frame_set(int(f),subframe=float(f%1));bpy.context.view_layer.update()
        head=rig.pose.bones['Head']
        for side in ['L','R']:
            wing=rig.pose.bones['Wing.'+side];target=head.matrix@head.bone.matrix_local.inverted()@wing.bone.head_local
            attachment_error=max(attachment_error,(wing.head-target).length);angles.append(wing.rotation_quaternion.angle)
        evalmesh=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        coords=np.array([v.co[:] for v in evalmesh.data.vertices]);assert np.isfinite(coords).all()
        assert np.max(np.linalg.norm(coords[weighted],axis=1))<4
        points.append(coords[weighted])
    assert attachment_error<1e-5,(name,attachment_error)
    if loop:assert np.max(np.abs(points[0]-points[-1]))<.005,(name,'loop seam')
    records.append({'clip':name,'samples':9,'wing_root_error_m':attachment_error,'max_baked_rotation_rad':max(angles)})
report={'weighted_vertices':len(weighted),'independent_bones':4,'clips':records,'bone_count':len(rig.data.bones)}
(OUT/'wing-verification.json').write_text(json.dumps(report,indent=2));print('ERPIN_WINGS_VERIFIED',json.dumps(report),flush=True)
