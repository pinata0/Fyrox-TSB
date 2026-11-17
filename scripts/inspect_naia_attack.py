"""Inspect the shipped Naia rig and disconnected hair islands without editing it."""
import bpy, json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/naia-attack'
OUT.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(ROOT / 'assets/characters/naia.blend'))
mesh = bpy.data.objects['naia_SkinnedMesh']
rig = bpy.data.objects['TSB_Rig']
adj = [[] for _ in mesh.data.vertices]
for edge in mesh.data.edges:
    a, b = edge.vertices
    adj[a].append(b); adj[b].append(a)
seen = set(); islands = []
for i in range(len(adj)):
    if i in seen: continue
    seen.add(i); stack = [i]; ids = []
    while stack:
        a = stack.pop(); ids.append(a)
        for b in adj[a]:
            if b not in seen: seen.add(b); stack.append(b)
    if len(ids) < 10: continue
    points = [mesh.data.vertices[j].co for j in ids]
    weights = {}
    for j in ids:
        for g in mesh.data.vertices[j].groups:
            name = mesh.vertex_groups[g.group].name
            weights[name] = weights.get(name, 0) + g.weight
    islands.append(dict(ids=ids, vertices=len(ids), bounds=[[min(p[k] for p in points), max(p[k] for p in points)] for k in range(3)], weights=weights))
report = dict(bones=[dict(name=b.name, parent=b.parent.name if b.parent else None, head=list(b.head_local), tail=list(b.tail_local)) for b in rig.data.bones], islands=islands,
              shapes=list(mesh.data.shape_keys.key_blocks.keys()), materials=[m.name for m in mesh.data.materials], objects=[o.name for o in bpy.context.scene.objects])
(OUT/'inspection.json').write_text(json.dumps(report, indent=2))
print(json.dumps({**report, 'islands':[{k:v for k,v in i.items() if k!='ids'} for i in islands]}, indent=2), flush=True)
