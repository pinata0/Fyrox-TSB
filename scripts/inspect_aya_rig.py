import bpy,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(root/'assets/characters/aya.blend'))
bpy.context.scene.frame_set(0)
rig=bpy.data.objects['TSB_Rig']
print('AYA_RIG',json.dumps({'end':bpy.context.scene.frame_end,'bones':[{'name':b.name,'head':list(b.head),'tail':list(b.tail)} for b in rig.data.bones],'meshes':[{'name':o.name,'vertices':len(o.data.vertices),'groups':[g.name for g in o.vertex_groups],'bounds':[[min(v.co[j] for v in o.data.vertices),max(v.co[j] for v in o.data.vertices)] for j in range(3)]} for o in bpy.context.scene.objects if o.type=='MESH']}))

body=bpy.data.objects["aya_SkinnedMesh"]
for group in ["Hips","Torso","Thigh.L","Thigh.R","Hair"]:
 vs=[v for v in body.data.vertices if any(body.vertex_groups[g.group].name==group and g.weight>.4 for g in v.groups)]
 print("WEIGHTS",group,len(vs),[[round(min(v.co[j] for v in vs),3),round(max(v.co[j] for v in vs),3)] for j in range(3)])
