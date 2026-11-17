import bpy, json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(root/'assets/characters/erpin.blend'))
rig=bpy.data.objects['TSB_Rig']; mesh=bpy.data.objects['erpin_SkinnedMesh']
bpy.context.scene.frame_set(0)
report={'bones':{},'wings':{},'objects':[(o.name,o.type) for o in bpy.context.scene.objects]}
for b in rig.data.bones:
    if b.name in ['Root','Hips','Torso','Head','mixamorig:Spine1','mixamorig:Spine2'] or 'Wing' in b.name:
        report['bones'][b.name]={'parent':b.parent.name if b.parent else None,'head':list(b.head_local),'tail':list(b.tail_local),'pose':list(rig.pose.bones[b.name].head)}
for mesh in [o for o in bpy.context.scene.objects if o.type=='MESH']:
    report['wings'][mesh.name]={'groups':[g.name for g in mesh.vertex_groups],'materials':[m.name for m in mesh.data.materials]}
    for side in ['L','R']:
        group=mesh.vertex_groups.get('Wing.'+side)
        if not group:continue
        ids=[v.index for v in mesh.data.vertices if any(g.group==group.index and g.weight>.5 for g in v.groups)]
        pts=[v.co for v in mesh.data.vertices if v.index in set(ids)]
        if not pts:continue
        report['wings'][mesh.name][side]={'count':len(ids),'bounds':[[min(p[j] for p in pts),max(p[j] for p in pts)] for j in range(3)]}
(root/'artifacts/erpin-motion/rig-before.json').write_text(json.dumps(report,indent=2))
print(json.dumps(report),flush=True)
