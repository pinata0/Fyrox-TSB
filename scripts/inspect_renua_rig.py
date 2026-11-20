import bpy,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(root/'assets/characters/renua.blend'))
s=bpy.context.scene;s.frame_set(0);r=bpy.data.objects['TSB_Rig']
report={'bones':[{ 'name':b.name,'head':list(b.head),'tail':list(b.tail)} for b in r.pose.bones], 'objects':[{ 'name':o.name,'type':o.type,'bounds':[list(v) for v in o.bound_box] if o.type=='MESH' else None,'groups':[g.name for g in o.vertex_groups] if o.type=='MESH' else []} for o in s.objects]}
(root/'artifacts/renua-skills/rig.json').write_text(json.dumps(report,indent=2))
print('RENUA_RIG',[(o.name,len(o.data.vertices)) for o in s.objects if o.type=='MESH'],flush=True)
