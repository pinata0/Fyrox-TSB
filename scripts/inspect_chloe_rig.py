import bpy,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(root/'assets/characters/chloe.blend'))
bpy.context.scene.frame_set(0)
r=bpy.data.objects['TSB_Rig']
data={'bones':[{ 'name':b.name,'parent':b.parent.name if b.parent else None,'head':list(b.head),'tail':list(b.tail),'rest':list(b.bone.head_local)} for b in r.pose.bones],'meshes':[{ 'name':o.name,'vertices':len(o.data.vertices),'groups':[g.name for g in o.vertex_groups],'bounds':[list(o.matrix_world@__import__('mathutils').Vector(c)) for c in o.bound_box]} for o in bpy.context.scene.objects if o.type=='MESH']}
(root/'artifacts/chloe-skills/rig.json').write_text(json.dumps(data,indent=2))
print('CHLOE_RIG',[(b['name'],b['parent'],b['head']) for b in data['bones'] if b['name'].startswith(('Sebastian','Hips','Head'))])
for g in bpy.data.objects['Sebastian'].vertex_groups:
 pts=[v.co for v in bpy.data.objects['Sebastian'].data.vertices if any(w.group==g.index and w.weight>.3 for w in v.groups)]
 print(g.name,[(min(v[i] for v in pts),max(v[i] for v in pts)) for i in range(3)] if pts else [])
