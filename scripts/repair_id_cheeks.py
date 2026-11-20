"""Repair Id's cheek controls without touching the model, rig or eye textures.
Run in Blender: --background --python scripts/repair_id_cheeks.py.
"""
import bpy,json,sys,hashlib
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from build_cheek_masks import smooth
A=ROOT/'assets/characters'
bpy.ops.wm.open_mainfile(filepath=str(A/'id.blend'))
mesh=bpy.data.objects['id_SkinnedMesh'].data
p=np.array([v.co[:] for v in mesh.vertices]);x,y,z=p.T
record=json.loads((A/'id-cheeks.json').read_text());rows=np.array(record['vertices'])
assert np.allclose(p,rows[:,:3],atol=1e-6)
image=bpy.data.images.load(str(A/'id-meshy-basecolor.png'),check_existing=False)
w,h=image.size;pixels=np.array(image.pixels[:],dtype=np.float32).reshape(h,w,4)
colors=np.zeros((len(p),3));count=np.zeros(len(p))
for loop,uv in zip(mesh.loops,mesh.uv_layers.active.data):
 u,v=uv.uv;i=loop.vertex_index;colors[i]+=pixels[min(h-1,max(0,int(v*h))),min(w-1,max(0,int(u*w))),:3];count[i]+=1
colors/=np.maximum(count[:,None],1);r,g,b=colors.T
cx=json.loads((A/'id-rig.json').read_text())['face']['center_x']
# The white sideburn sheets lie in front of the curved pink cheek. The old
# generic warm/white test admitted them; its eye guard also covered real skin.
skin=(r-g>.035)&(r-b>.025)&(r>.7)
front=np.interp(np.abs(x-cx),[0,.08,.11,.14,.17,.20],[-.217,-.202,-.191,-.177,-.157,-.126])
skin&=(np.abs(y-front)<.022)&(z>1.183)&(z<1.246)&(np.abs(x-cx)>.060)&(np.abs(x-cx)<.196)
mesh.calc_loop_triangles();tri=np.array([t.vertices[:] for t in mesh.loop_triangles])
# Keep complete triangles that touch the painted lower eyelid pinned.
eye=(p[tri,2].max(axis=1)>1.246)&(p[tri,2].min(axis=1)<1.36)&(np.abs(p[tri,0]-cx).min(axis=1)<.23)
eyes=np.zeros(len(p),dtype=bool);eyes[tri[eye].ravel()]=True;skin&=~eyes
rows[:,3:11]=0;rows[:,11]=rows[:,11].astype(int)|8;rows[skin,11]=0
adj=[[] for _ in p]
for edge in mesh.edges:
 a,b=edge.vertices;adj[a].append(b);adj[b].append(a)
centers=[];stats=[]
for side,sign in enumerate([-1,1]):
 target=np.array([cx+sign*.132,-.182,1.215])
 q=((x-target[0])/.084)**2+((z-target[2])/.037)**2
 weight=(1-smooth(np.sqrt(q)))*skin
 # Match the runtime coordinate merge so seams cannot move independently.
 gradient=np.zeros_like(p)
 for i in np.flatnonzero(weight>0):
  ns=adj[i];delta=p[ns]-p[i]
  gradient[i]=np.linalg.lstsq(np.vstack([delta,np.eye(3)*.005]),np.r_[weight[ns]-weight[i],np.zeros(3)],rcond=None)[0]
 rows[:,3+side]=np.round(weight,6);rows[:,5+3*side:8+3*side]=np.round(gradient,5)
 apex=int(np.argmax(weight));centers.append(p[apex].tolist())
 stats.append(dict(apex_vertex=apex,apex_weight=float(weight[apex]),moving_vertices=int(sum(weight>.001)),core_vertices=int(sum(weight>.8)),core_height=float(np.ptp(z[weight>.8])),core_width=float(np.ptp(x[weight>.8]))))
 assert weight[apex]>.85 and sum(weight>.2)>25,(side,stats[-1])
assert not rows[~skin,3:11].any()
record.update(vertices=rows.tolist(),centers=centers,hit_radius=.10,stats=stats,
 skin_domain_revision=2,skin_domain_note='Measured lower pink cheek; white sideburns and complete eyelid triangles pinned')
(A/'id-cheeks.json').write_text(json.dumps(record,separators=(',',':')))
manifest_path=A/'manifest.json'
original=manifest_path.read_bytes();manifest=json.loads(original)
hero=next(row for row in manifest if row['slug']=='id')
hero['sha256']['cheeks']=hashlib.sha256((A/'id-cheeks.json').read_bytes()).hexdigest()
hero['cheek_refinement'].update(centers=centers,hit_radius_m=.10,skin_domain_revision=2)
assert manifest_path.read_bytes()==original,'manifest changed during repair'
manifest_path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print('ID_CHEEK_SKIN_OK',centers,stats,flush=True)
