"""Seal tiny voxel/decimation pinholes without moving vertices or changing UV islands."""
import bpy,bmesh,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
DEST=ROOT/'artifacts/hand-mesh-polish/characters'
slugs=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else 'vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
for slug in slugs:
    bpy.ops.wm.open_mainfile(filepath=str(DEST/f'{slug}.blend'));filled=0
    for obj in list(bpy.data.objects):
        if obj.type!='MESH' or '_Refined' not in obj.name:continue
        bm=bmesh.new();bm.from_mesh(obj.data)
        boundary_vertices={v for edge in bm.edges if edge.is_boundary for v in edge.verts}
        if boundary_vertices:bmesh.ops.remove_doubles(bm,verts=list(boundary_vertices),dist=.0003)
        boundaries=[edge for edge in bm.edges if edge.is_boundary]
        assert len(boundaries)<100,(slug,obj.name,'unexpected large boundary')
        if boundaries:
            result=bmesh.ops.holes_fill(bm,edges=boundaries,sides=0)
            for face in result['faces']:
                assert max((a.co-b.co).length for a in face.verts for b in face.verts)<.02
            filled+=len(result['faces'])
        bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(obj.data);bm.free()
        for v in obj.data.vertices:
            values=sorted([(g.group,g.weight) for g in v.groups if g.weight>1e-6],key=lambda x:x[1],reverse=True)[:4]
            total=sum(w for _,w in values)
            for group in list(v.groups):obj.vertex_groups[group.group].remove([v.index])
            for group,w in values:obj.vertex_groups[group].add([v.index],w/total,'REPLACE')
    bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(DEST/f'{slug}.blend'))
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and any(m.type=='ARMATURE' and m.object==rig for m in o.modifiers)]
    export_fbx(body,rig,DEST/f'{slug}.fbx',extras)
    print('REFINED_SURFACES_SEALED',slug,filled,flush=True)
