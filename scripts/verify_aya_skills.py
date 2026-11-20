"""Verify actual skinned cloth deformation, bounded weights and original clip keys."""
import bpy,json,math
from pathlib import Path
from mathutils import Matrix
ROOT=Path(__file__).resolve().parents[1];WORK=ROOT/'artifacts/aya-skills'
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/aya.blend'))
rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['aya_SkinnedMesh'];scene=bpy.context.scene
chains=[p for p in rig.pose.bones if p.name.startswith('AyaSkirt.')];assert len(chains)==16
indices=[v.index for v in body.data.vertices if any(body.vertex_groups[g.group].name.startswith('AyaSkirt.') and g.weight>.01 for g in v.groups)]
assert len(indices)>500
for v in body.data.vertices:
    assert all(math.isfinite(g.weight) and 0<=g.weight<=1 for g in v.groups)
    assert abs(sum(g.weight for g in v.groups)-1)<.002,(v.index,sum(g.weight for g in v.groups))
def points():
    bpy.context.view_layer.update();mesh=body.evaluated_get(bpy.context.evaluated_depsgraph_get())
    return [mesh.matrix_world@mesh.data.vertices[i].co for i in indices]
checks=[]
for frame in [578,597,615,685,712,730]:
    scene.frame_set(frame);moving=points()
    for p in chains:p.matrix_basis=Matrix.Identity(4)
    rigid=points();delta=max((a-b).length for a,b in zip(moving,rigid))
    assert delta>.003,(frame,delta)
    checks.append({'frame':frame,'max_cloth_only_displacement_m':delta})
source=json.loads((ROOT/'assets/characters/skill/aya.json').read_text())
assert [len(c['samples']) for c in source['clips']]==[95,111]
verification=json.loads((WORK/'bake-verification.json').read_text());assert verification['old_keys_preserved'] and verification['geometry_preserved']
(WORK/'cloth-verification.json').write_text(json.dumps({'weighted_vertices':len(indices),'normalized_weights':True,'source_frames':[95,111],'checks':checks},indent=2))
print('AYA_CLOTH_VERIFIED',checks,flush=True)
