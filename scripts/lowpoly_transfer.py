"""Geometry, skin and texture transfer helpers for the supplied remeshes."""
import bpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.kdtree import KDTree

def points(obj):
    return np.array([v.co[:] for v in obj.data.vertices])

def barycentric(p, triangle):
    a,b,c=np.moveaxis(triangle,-2,0)
    u=b-a;v=c-a;w=p-a
    uu=np.sum(u*u,axis=-1);uv=np.sum(u*v,axis=-1);vv=np.sum(v*v,axis=-1)
    wu=np.sum(w*u,axis=-1);wv=np.sum(w*v,axis=-1)
    den=np.maximum(uu*vv-uv*uv,1e-20)
    y=(vv*wu-uv*wv)/den;z=(uu*wv-uv*wu)/den
    weights=np.maximum(np.stack([1-y-z,y,z],axis=-1),0)
    return weights/np.maximum(weights.sum(-1,keepdims=True),1e-20)

class Surface:
    def __init__(self,obj):
        self.obj=obj;self.p=points(obj);mesh=obj.data;mesh.calc_loop_triangles()
        self.tri=np.array([t.vertices[:] for t in mesh.loop_triangles],dtype=int)
        self.tree=BVHTree.FromPolygons(self.p.tolist(),self.tri.tolist(),all_triangles=True)
        self.uv=np.array([[mesh.uv_layers.active.data[i].uv[:] for i in t.loops] for t in mesh.loop_triangles])
        self.used=np.unique(self.tri)

    def nearest(self,p):
        result=[self.tree.find_nearest(v) for v in p]
        tri=np.array([r[2] for r in result]);q=np.array([r[0] for r in result])
        return self.tri[tri],barycentric(q,self.p[self.tri[tri]]),np.array([r[3] for r in result]),tri

    def uv_at(self,p):
        _,w,_,tri=self.nearest(p)
        return np.sum(self.uv[tri]*w[:,:,None],axis=1)

def interpolate(values,indices,weights):
    return np.einsum('ij,ij...->i...',weights,values[indices])

def transfer_normals(old,new,surface):
    _,weights,_,tri=surface.nearest(points(new))
    source=np.array([[old.data.corner_normals[i].vector[:] for i in t.loops] for t in old.data.loop_triangles])
    normal=np.einsum('ij,ijk->ik',weights,source[tri])
    normal/=np.maximum(np.linalg.norm(normal,axis=1,keepdims=True),1e-9)
    # Opposite sides of a paper-thin wing can be equally close. Retain the
    # destination surface orientation instead of importing a reversed normal.
    geometric=np.array([v.normal[:] for v in new.data.vertices])
    reverse=np.sum(normal*geometric,axis=1)<.1
    normal[reverse]=geometric[reverse]
    new.data.normals_split_custom_set_from_vertices(normal.tolist())
    return int(sum(reverse))

def append_mesh(path):
    with bpy.data.libraries.load(str(path),link=False) as (available,loaded):
        loaded.objects=list(available.objects)
    meshes=[o for o in loaded.objects if o and o.type=='MESH']
    assert len(meshes)==1,(path,[o.name for o in meshes])
    obj=meshes[0];bpy.context.collection.objects.link(obj)
    bpy.context.view_layer.update()
    obj.data.transform(obj.matrix_world);obj.matrix_world.identity()
    return obj

def transformed(obj,fit):
    p=points(obj)@np.array(fit['rotation']).T*fit['scale']+fit['offset']
    obj.data.vertices.foreach_set('co',p.astype(np.float32).ravel());obj.data.update()
    return p

def trim_unused_arms(new,old,surface):
    """Current bodies retain unused vertices where the separate arms replaced faces."""
    unused=np.setdiff1d(np.arange(len(surface.p)),surface.used)
    if not len(unused):return 0
    tree=KDTree(len(unused))
    for j,i in enumerate(unused):tree.insert(surface.p[i],j)
    tree.balance()
    p=points(new)
    centers=np.array([p[list(f.vertices)].mean(0) for f in new.data.polygons])
    distance=surface.nearest(centers)[2]
    missing=np.array([tree.find(c)[2] for c in centers])
    remove=(missing+.004<distance)&(distance>.009)
    import bmesh
    bm=bmesh.new();bm.from_mesh(new.data);bm.faces.ensure_lookup_table()
    bmesh.ops.delete(bm,geom=[f for f in bm.faces if remove[f.index]],context='FACES')
    bmesh.ops.delete(bm,geom=[v for v in bm.verts if not v.link_faces],context='VERTS')
    bm.to_mesh(new.data);bm.free()
    return int(sum(remove))

def transfer_skin_and_shapes(old,new,surface):
    p=points(new);indices,w,distance,_=surface.nearest(p)
    groups=[g.name for g in old.vertex_groups]
    values=np.zeros((len(surface.p),len(groups)),dtype=np.float32)
    for v in old.data.vertices:
        for g in v.groups:values[v.index,g.group]=g.weight
    result=interpolate(values,indices,w)
    new.vertex_groups.clear()
    new_groups=[new.vertex_groups.new(name=n) for n in groups]
    for i,row in enumerate(result) if groups else []:
        order=np.argsort(row)[-4:];total=row[order].sum()
        assert total>1e-5,(old.name,'unweighted',i)
        for j in order:
            if row[j]>1e-7:new_groups[j].add([i],float(row[j]/total),'REPLACE')
    if old.data.shape_keys:
        basis=points(old)
        for key in old.data.shape_keys.key_blocks:
            delta=np.array([v.co[:] for v in key.data])-basis
            shape=new.shape_key_add(name=key.name,from_mix=False)
            shape.data.foreach_set('co',(p+interpolate(delta,indices,w)).astype(np.float32).ravel())
            shape.value=0;shape.slider_min=key.slider_min;shape.slider_max=key.slider_max
        for name in ['blink_normal_L','blink_normal_R','blink_weight_L','blink_weight_R']:
            attr=old.data.attributes.get(name)
            if attr is None:continue
            field='vector' if attr.data_type=='FLOAT_VECTOR' else 'value'
            values=np.array([getattr(v,field)[:] if field=='vector' else getattr(v,field) for v in attr.data])
            dst=new.data.attributes.new(name,attr.data_type,'POINT')
            dst.data.foreach_set(field,interpolate(values,indices,w).astype(np.float32).ravel())
    return indices,w,distance

def uv_projection(new,surface,size=1024):
    """Rasterize new UVs and project each texel onto the authored source surface."""
    new.data.calc_loop_triangles();p=points(new)
    positions=np.zeros((size,size,3),dtype=np.float32);covered=np.zeros((size,size),dtype=bool)
    for t in new.data.loop_triangles:
        uv=np.array([new.data.uv_layers.active.data[i].uv[:] for i in t.loops])*size-.5
        low=np.maximum(np.floor(uv.min(0)).astype(int),0);high=np.minimum(np.ceil(uv.max(0)).astype(int),size-1)
        if np.any(high<low):continue
        x,y=np.meshgrid(np.arange(low[0],high[0]+1),np.arange(low[1],high[1]+1))
        a,b,c=uv;den=(b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1])
        if abs(den)<1e-10:continue
        w0=((b[1]-c[1])*(x-c[0])+(c[0]-b[0])*(y-c[1]))/den
        w1=((c[1]-a[1])*(x-c[0])+(a[0]-c[0])*(y-c[1]))/den;w2=1-w0-w1
        inside=(w0>=-1e-6)&(w1>=-1e-6)&(w2>=-1e-6)
        x=x[inside];y=y[inside];w=np.stack([w0[inside],w1[inside],w2[inside]],axis=1)
        positions[y,x]=w@p[list(t.vertices)];covered[y,x]=True
    uv=surface.uv_at(positions[covered]);return covered,uv

def reproject_image(source,path,covered,uv):
    image=bpy.data.images.load(str(source),check_existing=False)
    width,height=image.size
    pixels=np.empty(width*height*4,dtype=np.float32);image.pixels.foreach_get(pixels);pixels=pixels.reshape(height,width,4)
    xy=np.clip(uv*np.array([width,height])-.5,[0,0],[width-1,height-1]);low=np.floor(xy).astype(int);high=np.minimum(low+1,[width-1,height-1]);f=xy-low
    x,y=low.T;xx,yy=high.T;fx,fy=f.T
    color=(pixels[y,x]*(1-fx[:,None])+pixels[y,xx]*fx[:,None])*(1-fy[:,None])+(pixels[yy,x]*(1-fx[:,None])+pixels[yy,xx]*fx[:,None])*fy[:,None]
    size=covered.shape[0];out=np.zeros((size,size,4),dtype=np.float32);out[covered]=color
    mask=covered.copy()
    # Extend islands into their margins so mipmaps do not introduce black seams.
    for _ in range(12):
        expanded=mask.copy()
        for dy,dx in [(0,1),(0,-1),(1,0),(-1,0)]:
            neighbor=np.roll(mask,(dy,dx),(0,1));fill=~expanded&neighbor
            out[fill]=np.roll(out,(dy,dx),(0,1))[fill];expanded|=fill
        mask=expanded
    out[:,:,3]=1
    result=bpy.data.images.new(path.stem,width=size,height=size,alpha=False)
    result.pixels.foreach_set(out.ravel());result.filepath_raw=str(path);result.file_format='PNG';result.save()
    bpy.data.images.remove(image)
    return result

def reduce_mesh(obj,budget):
    obj.data.calc_loop_triangles();before=len(obj.data.loop_triangles)
    if before<=budget:return before,before
    assert not obj.data.shape_keys,obj.name
    bpy.ops.object.select_all(action='DESELECT');obj.select_set(True);bpy.context.view_layer.objects.active=obj
    mod=obj.modifiers.new('Low polygon runtime budget','DECIMATE');mod.ratio=budget/before;mod.use_collapse_triangulate=True
    # Apply in rest pose before the armature: skin groups and UVs interpolate with the surface.
    bpy.ops.object.modifier_move_up(modifier=mod.name)
    bpy.ops.object.modifier_apply(modifier=mod.name)
    for vertex in obj.data.vertices:
        row=sorted([(g.group,g.weight) for g in vertex.groups if g.weight>1e-7],key=lambda x:-x[1])[:4]
        total=sum(w for _,w in row)
        if obj.vertex_groups:
            assert total>0,obj.name
            for group in list(vertex.groups):obj.vertex_groups[group.group].remove([vertex.index])
            for group,weight in row:obj.vertex_groups[group].add([vertex.index],weight/total,'REPLACE')
    obj.data.calc_loop_triangles();return before,len(obj.data.loop_triangles)
