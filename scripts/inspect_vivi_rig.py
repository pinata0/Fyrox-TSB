import bpy,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(root/'assets/characters/vivi.blend'))
rig=bpy.data.objects['TSB_Rig'];bpy.context.scene.frame_set(0)
print('VIVI_BONES',[(b.name,list(b.head_local),list(b.tail_local)) for b in rig.data.bones])
for o in bpy.context.scene.objects:
 if o.type!='MESH':continue
 print('MESH',o.name,len(o.data.vertices),'BOUNDS',[(min(v.co[i] for v in o.data.vertices),max(v.co[i] for v in o.data.vertices)) for i in range(3)])
 print('GROUPS',[(g.name,sum(1 for v in o.data.vertices if any(x.group==g.index and x.weight>.1 for x in v.groups))) for g in o.vertex_groups if 'Wing' in g.name or 'Skirt' in g.name or 'Hair' in g.name or 'Tail' in g.name])
body=bpy.data.objects['vivi_SkinnedMesh'];parent=list(range(len(body.data.vertices)))
def find(i):
 while parent[i]!=i:parent[i]=parent[parent[i]];i=parent[i]
 return i
for e in body.data.edges:
 a,b=map(find,e.vertices);parent[a]=b
groups={}
for v in body.data.vertices:groups.setdefault(find(v.index),[]).append(v.index)
report=[]
for indices in sorted(groups.values(),key=len,reverse=True):
 if len(indices)<30:continue
 vs=[body.data.vertices[i].co for i in indices]
 bounds=[[round(min(v[j] for v in vs),3),round(max(v[j] for v in vs),3)] for j in range(3)]
 report.append({'count':len(indices),'bounds':bounds,'indices':indices})
print('COMPONENTS',[(i,g['count'],g['bounds']) for i,g in enumerate(report)])
(root/'artifacts/vivi-skills/components.json').write_text(json.dumps(report))
