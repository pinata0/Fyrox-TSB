import bpy,sys,json,numpy as np
from pathlib import Path
from mathutils.bvhtree import BVHTree
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
for source in ['artifacts/cheek-isolation/before','assets/characters']:
    bpy.ops.wm.open_mainfile(filepath=str(ROOT/source/'chloe.blend'))
    mesh=bpy.data.objects['chloe_SkinnedMesh'].data
    coords=np.array([v.co[:] for v in mesh.vertices]);basis=np.array([v.co[:] for v in mesh.shape_keys.key_blocks[0].data]);mesh.calc_loop_triangles()
    triangles=np.array([t.vertices[:] for t in mesh.loop_triangles]);p=coords[triangles]
    areas=np.linalg.norm(np.cross(p[:,1]-p[:,0],p[:,2]-p[:,0]),axis=1)/2
    keys=[tuple(sorted(tuple(np.round(v,6)) for v in tri)) for tri in p]
    print(source,'vertices',len(coords),'tris',len(triangles),'basisdelta',np.abs(basis-coords).max(),'duplicate_triangles',len(keys)-len(set(keys)),'tiny',sum(areas<1e-10),'customnormals',mesh.has_custom_normals,flush=True)
    if 'before' in source:
        tree=BVHTree.FromPolygons([Vector(v) for v in coords],triangles.tolist(),all_triangles=True)
        original_p=p.copy();original_uv=np.array([[mesh.uv_layers.active.data[i].uv[:] for i in t.loops] for t in mesh.loop_triangles])
    else:
        samples=[tree.find_nearest(Vector(v)) for v in coords[np.unique(triangles)]]
        print('nearest_original_distance',np.quantile([s[3] for s in samples],[.5,.95,.99,1]),flush=True)
        centers=p.mean(1);normals=np.cross(p[:,1]-p[:,0],p[:,2]-p[:,0]);normals/=np.maximum(np.linalg.norm(normals,axis=1)[:,None],1e-12)
        matched=[tree.find_nearest(Vector(v)) for v in centers]
        dots=np.array([np.dot(n,s[1]) for n,s in zip(normals,matched)])
        print('reversed_triangles',sum((dots<-.5)&(areas>1e-10)),'center_distances',np.quantile([s[3] for s in matched],[.5,.95,.99,1]),flush=True)
        selected=np.array([s[2] for s in matched]);a=original_p[selected];u=original_uv[selected]
        e=a[:,1]-a[:,0];f=a[:,2]-a[:,0];v=p-a[:,None,0]
        ee=(e*e).sum(1);ff=(f*f).sum(1);ef=(e*f).sum(1);ve=(v*e[:,None]).sum(2);vf=(v*f[:,None]).sum(2);denom=ee*ff-ef*ef
        b1=(ff[:,None]*ve-ef[:,None]*vf)/denom[:,None];b2=(ee[:,None]*vf-ef[:,None]*ve)/denom[:,None]
        expected=(1-b1-b2)[:,:,None]*u[:,None,0]+b1[:,:,None]*u[:,None,1]+b2[:,:,None]*u[:,None,2]
        actual=np.array([[mesh.uv_layers.active.data[i].uv[:] for i in t.loops] for t in mesh.loop_triangles])
        print('UV errors',np.quantile(np.abs(actual-expected),[.5,.95,.99,1]),'over1pixel',sum(np.max(np.abs(actual-expected),axis=(1,2))>1/2048),flush=True)
