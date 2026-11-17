"""Conforming midpoint refinement with exact UV/key/weight interpolation.

Each child triangle stays inside its original triangle. Per-corner UVs retain
seams; interpolated custom normals retain the original smooth shading.
"""
import bpy
import numpy as np

def refine(body,face):
    original=body.data;original.calc_loop_triangles()
    names=[k.name for k in original.shape_keys.key_blocks]
    shapes=np.array([[v.co[:] for v in k.data] for k in original.shape_keys.key_blocks],dtype=np.float32)
    normals=np.array([v.normal[:] for v in original.vertices],dtype=np.float32)
    groups=[{g.group:g.weight for g in v.groups} for v in original.vertices]
    group_names=[group.name for group in body.vertex_groups]
    triangles=np.array([t.vertices[:] for t in original.loop_triangles],dtype=np.int32)
    uvs=np.array([[original.uv_layers.active.data[i].uv[:] for i in t.loops] for t in original.loop_triangles],dtype=np.float32)
    materials=[t.material_index for t in original.loop_triangles]
    patterns={
        0:[(0,1,2)],1:[(0,3,2),(3,1,2)],2:[(1,4,0),(4,2,0)],4:[(2,5,1),(5,0,1)],
        3:[(3,1,4),(0,3,2),(3,4,2)],6:[(4,2,5),(1,4,0),(4,5,0)],5:[(5,0,3),(2,5,1),(5,3,1)],
        7:[(0,3,5),(3,1,4),(5,4,2),(3,4,5)],
    }
    for iteration in range(4):
        coords=shapes[0];x,y,z=coords.T
        roi=(np.abs(x-face['center_x'])<.39)&(z>face['mouth_y']-.14)&(z<face['eye_y']+.23)&(y<-face['front']+.24)
        edges=np.unique(np.sort(np.concatenate([triangles[:,[0,1]],triangles[:,[1,2]],triangles[:,[2,0]]]),axis=1),axis=0)
        edges=edges[(roi[edges].any(1))&(np.linalg.norm(coords[edges[:,0]]-coords[edges[:,1]],axis=1)>.012)]
        if not len(edges):break
        midpoint={tuple(edge):len(coords)+i for i,edge in enumerate(edges)}
        shapes=np.concatenate([shapes,(shapes[:,edges[:,0]]+shapes[:,edges[:,1]])*.5],axis=1)
        new_normals=(normals[edges[:,0]]+normals[edges[:,1]])*.5
        new_normals/=np.maximum(np.linalg.norm(new_normals,axis=1)[:,None],1e-12)
        normals=np.concatenate([normals,new_normals])
        for a,b in edges:
            weights={k:(groups[a].get(k,0)+groups[b].get(k,0))*.5 for k in groups[a].keys()|groups[b].keys()}
            weights=dict(sorted(weights.items(),key=lambda item:item[1],reverse=True)[:4]);total=sum(weights.values())
            groups.append({k:v/total for k,v in weights.items()})
        new_tri=[];new_uv=[];new_mat=[]
        for tri,uv,material in zip(triangles,uvs,materials):
            mids=[midpoint.get(tuple(sorted((int(tri[a]),int(tri[b])))),-1) for a,b in [(0,1),(1,2),(2,0)]]
            mask=sum((1<<i) for i,m in enumerate(mids) if m>=0)
            vertices=[*tri,*mids];corners=np.vstack([uv,(uv[0]+uv[1])*.5,(uv[1]+uv[2])*.5,(uv[2]+uv[0])*.5])
            for pattern in patterns[mask]:
                new_tri.append([vertices[i] for i in pattern]);new_uv.append(corners[list(pattern)]);new_mat.append(material)
        triangles=np.array(new_tri,dtype=np.int32);uvs=np.array(new_uv,dtype=np.float32);materials=new_mat
    mesh=bpy.data.meshes.new(original.name+'_CheekRefined');mesh.from_pydata(shapes[0].tolist(),[],triangles.tolist());mesh.update()
    for material in original.materials:mesh.materials.append(material)
    uv=mesh.uv_layers.new(name='UVMap');uv.data.foreach_set('uv',uvs.ravel())
    for polygon,material in zip(mesh.polygons,materials):polygon.use_smooth=True;polygon.material_index=material
    mesh.normals_split_custom_set_from_vertices(normals.tolist())
    body.data=mesh
    body.vertex_groups.clear()
    for name in group_names:body.vertex_groups.new(name=name)
    for i,name in enumerate(names):
        key=body.shape_key_add(name=name,from_mix=False);key.value=0;key.data.foreach_set('co',shapes[i].ravel())
    for material in mesh.materials:
        animation=material.node_tree.animation_data
        if animation:
            for curve in animation.drivers:
                for variable in curve.driver.variables:
                    for target in variable.targets:
                        if target.id==original.shape_keys:target.id=mesh.shape_keys
    for i,weights in enumerate(groups):
        for group,weight in weights.items():body.vertex_groups[group].add([i],weight,'REPLACE')
    return mesh
