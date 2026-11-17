import bpy,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(root/'assets/characters/erpin.blend'))
o=bpy.data.objects['erpin_SkinnedMesh'];mesh=o.data
import numpy as np
material=mesh.materials[0]
image=next(n.image for n in material.node_tree.nodes if n.type=='TEX_IMAGE' and n.image.name==material['meshy_base_texture'])
w,h=image.size;pixels=np.array(image.pixels[:]).reshape(h,w,4)
uv=np.array([l.uv[:] for l in mesh.uv_layers.active.data]);xy=np.clip((uv*np.array([w,h])).astype(int),[0,0],[w-1,h-1])
colors=np.zeros((len(mesh.vertices),3));counts=np.zeros(len(mesh.vertices));indices=np.array([l.vertex_index for l in mesh.loops])
np.add.at(colors,indices,pixels[xy[:,1],xy[:,0],:3]);np.add.at(counts,indices,1);colors/=np.maximum(1,counts[:,None])
points=np.array([v.co[:] for v in mesh.vertices]);r,g,b=colors.T
mask=(points[:,1]>.18)&(points[:,2]>.80)&(points[:,2]<1.4)&(abs(points[:,0])>.15)&(b>r*.88)&(g>r*.92)&(r>.35)
np.savez(root/'artifacts/erpin-motion/vertices.npz',points=points,colors=colors,mask=mask)
parents=list(range(len(mesh.vertices)))
def find(i):
 while parents[i]!=i:parents[i]=parents[parents[i]];i=parents[i]
 return i
for e in mesh.edges:
 a,b=e.vertices
 if mask[a] and mask[b]:parents[find(a)]=find(b)
components={}
for v in mesh.vertices:
 if mask[v.index]:components.setdefault(find(v.index),[]).append(v.index)
rows=[]
for ids in components.values():
 if len(ids)<40:continue
 pts=[mesh.vertices[i].co for i in ids]
 weights={}
 for i in ids:
  for g in mesh.vertices[i].groups:
   n=o.vertex_groups[g.group].name;weights[n]=weights.get(n,0)+g.weight/len(ids)
 rows.append({'count':len(ids),'first':ids[0],'bounds':[[round(min(p[j] for p in pts),3),round(max(p[j] for p in pts),3)] for j in range(3)],'weights':weights})
(root/'artifacts/erpin-motion/components.json').write_text(json.dumps(rows,indent=2))
print(json.dumps(rows),flush=True)
