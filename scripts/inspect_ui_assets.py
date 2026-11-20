import bpy,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
out=root/'artifacts/ui-skills';out.mkdir(parents=True,exist_ok=True)
result={}
for name in ['ui','ui-weapon']:
 bpy.ops.wm.open_mainfile(filepath=str(root/f'assets/characters/{name}.blend'))
 result[name]=[{'name':o.name,'type':o.type,'parent':o.parent.name if o.parent else None,'bounds':[list(min((o.matrix_world@v.co)[i] for v in o.data.vertices) for i in range(3)),list(max((o.matrix_world@v.co)[i] for v in o.data.vertices) for i in range(3))] if o.type=='MESH' else None,'bones':[{'name':b.name,'parent':b.parent.name if b.parent else None,'head':list(b.head_local)} for b in o.data.bones] if o.type=='ARMATURE' else None,'groups':[v.name for v in o.vertex_groups] if o.type=='MESH' else None} for o in bpy.context.scene.objects]
(out/'assets.json').write_text(json.dumps(result,indent=2))
print('UI_INSPECTION_OK')
