import bpy,json
from pathlib import Path
P=Path(__file__).resolve().parents[1];bpy.ops.wm.open_mainfile(filepath=str(P/'assets/characters/chloe.blend'))
o=bpy.data.objects['Sebastian'];o.data.calc_loop_triangles();uv=o.data.uv_layers.active.data
m=o.data.materials[0];images=[n.image for n in m.node_tree.nodes if n.type=='TEX_IMAGE' and n.image];print('IMAGES',[(i.name,i.size[:],i.filepath) for i in images])
im=images[0];w,h=im.size;px=list(im.pixels);pts=[]
for t in o.data.loop_triangles:
 v=sum((o.data.vertices[i].co for i in t.vertices),__import__('mathutils').Vector())/3
 if not (.10<abs(v.x)<.55 and 1.72<v.z<2.12 and v.y<.40):continue
 u=sum((uv[i].uv for i in t.loops),__import__('mathutils').Vector((0,0)))/3
 idx=(int(u.y*h)%h*w+int(u.x*w)%w)*4;r,g,b=px[idx:idx+3]
 if max(r,g,b)<.12:pts.append(list(v))
print('DARK',json.dumps(pts))
(P/'artifacts/chloe-skills/eye-points.json').write_text(json.dumps(pts))
