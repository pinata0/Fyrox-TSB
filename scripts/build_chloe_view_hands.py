"""Cut two closed Sebastian limbs from his own mesh, retaining original UVs."""
import bpy,bmesh,json,os
from pathlib import Path
from mathutils import Vector
P=Path(__file__).resolve().parents[1]
A=Path(os.environ.get('TSB_CHARACTER_ASSET_DIR',P/'assets/characters'))
bpy.ops.wm.open_mainfile(filepath=str(A/'chloe.blend'))
source=bpy.data.objects['Sebastian'];parts=[]
for side,sign in [('L',-1),('R',1)]:
    bm=bmesh.new();bm.from_mesh(source.data)
    bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.00005)
    for co,normal in [((sign*.60,0,0),(sign,0,0)),((0,0,1.66),(0,0,-1)),((0,.12,0),(0,1,0))]:
        bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),plane_co=co,plane_no=normal,clear_inner=True,dist=.00001)
    boundary=[e for e in bm.edges if e.is_boundary]
    caps=bmesh.ops.holes_fill(bm,edges=boundary,sides=0)['faces']
    uv=bm.loops.layers.uv.active
    for face in caps:
        face.material_index=1
        for loop in face.loops:loop[uv].uv=(loop.vert.co.y,loop.vert.co.z)
    bmesh.ops.triangulate(bm,faces=list(bm.faces));bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.normal_update()
    for _ in range(3):
        boundary=[e for e in bm.edges if e.is_boundary]
        if not boundary:break
        new=bmesh.ops.holes_fill(bm,edges=boundary,sides=0)['faces']
        for face in new:
            face.material_index=1
            for loop in face.loops:loop[uv].uv=(loop.vert.co.y,loop.vert.co.z)
        bmesh.ops.triangulate(bm,faces=list(new))
    # Intersecting cut planes may leave a non-planar boundary that holes_fill
    # cannot triangulate. Close each connected loop with an explicit cap fan.
    for repair in range(4):
        pending=set(e for e in bm.edges if e.is_boundary)
        while pending:
            seed=pending.pop();component={seed};vertices=set(seed.verts);changed=True
            while changed:
                changed=False
                for edge in list(pending):
                    if any(v in vertices for v in edge.verts):
                        component.add(edge);vertices.update(edge.verts);pending.remove(edge);changed=True
            center=bm.verts.new(sum((v.co for v in vertices),Vector())/len(vertices))
            for edge in component:
                face=bm.faces.new((edge.verts[1],edge.verts[0],center));face.material_index=1
                for loop in face.loops:loop[uv].uv=(loop.vert.co.y,loop.vert.co.z)
        bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.normal_update()
    # Discard isolated cut edges, which have no renderable faces.
    # Wire edges are ignored by the triangle export.
    assert all(e.calc_length()<.025 for e in bm.edges if e.is_boundary), 'uncapped large cut'
    lo=Vector(tuple(min(v.co[k] for v in bm.verts) for k in range(3)));hi=Vector(tuple(max(v.co[k] for v in bm.verts) for k in range(3)));center=(lo+hi)*.5;scale=.88/(hi.z-lo.z)
    for cap in [False,True]:
        vertices=[];triangles=[]
        for face in bm.faces:
            if (face.material_index==1)!=cap:continue
            tri=[]
            for loop in face.loops:
                v=loop.vert.co;p=Vector((v.x-center.x,-(v.y-center.y),-(v.z-lo.z)))*scale;t=loop[uv].uv
                tri.append(len(vertices));vertices.append([*p,t.x,1-t.y])
            triangles.append(tri)
        parts.append({'side':side,'cap':cap,'vertices':vertices,'triangles':triangles})
    bm.free()
(A/'chloe-view-hands.json').write_text(json.dumps(parts,separators=(',',':')))
print('CHLOE_CLOSED_VIEW_HANDS_OK',[(p['side'],p['cap'],len(p['triangles'])) for p in parts],flush=True)
