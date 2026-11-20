import bpy,sys,json
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from lowpoly_transfer import Surface,points
WORK=ROOT/'artifacts/lowpoly-import'
slug='vivi'
bpy.ops.wm.open_mainfile(filepath=str(WORK/'characters'/f'{slug}.blend'))
body=bpy.data.objects[slug+'_SkinnedMesh']
print('NEW_NORMALS',body.data.has_custom_normals,flush=True)
with bpy.data.libraries.load(str(WORK/'before'/f'{slug}.blend'),link=False) as (_,loaded):loaded.objects=[slug+'_SkinnedMesh']
old=loaded.objects[0];surface=Surface(old)
indices,w,distance,tri=surface.nearest(points(body))
normal=np.array([[old.data.corner_normals[i].vector[:] for i in t.loops] for t in old.data.loop_triangles])
normal=np.einsum('ij,ijk->ik',w,normal[tri]);normal/=np.maximum(np.linalg.norm(normal,axis=1,keepdims=True),1e-8)
current=np.array([v.normal[:] for v in body.data.vertices]);angles=np.degrees(np.arccos(np.clip(np.sum(normal*current,axis=1),-1,1)))
print('NORMAL_DIFFERENCE',np.quantile(angles,[.5,.9,.99]),flush=True)
print('OLD_NORMALS',old.data.has_custom_normals,flush=True)
