"""Refine facial topology while interpolating UVs, all facial keys and skin weights."""
import bpy,bmesh,json,sys,shutil
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
from refine_face_triangles import refine
ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/cheek-isolation';BACKUP=OUT/'before'
SLUGS='vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
for slug in (sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS):
    for suffix in ['.blend','.fbx','.morphs.json','-rig.json']:
        source=ASSETS/f'{slug}{suffix}';dest=BACKUP/source.name
        if not dest.exists():shutil.copy2(source,dest)
    bpy.ops.wm.open_mainfile(filepath=str(BACKUP/f'{slug}.blend'))
    for image in bpy.data.images:
        if image.source=='FILE':
            original=ASSETS/Path(image.filepath).name
            if original.is_file():image.filepath=str(original)
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];mesh=body.data;rig=bpy.data.objects['TSB_Rig']
    record=json.loads((BACKUP/f'{slug}-rig.json').read_text());face=record['face']
    old_vertices=len(mesh.vertices);old_keys=[k.name for k in mesh.shape_keys.key_blocks]
    for key in mesh.shape_keys.key_blocks:key.value=0
    mesh=refine(body,face)
    for vertex in mesh.vertices:
        groups=sorted([(g.group,g.weight) for g in vertex.groups],key=lambda a:a[1],reverse=True)
        if len(groups)>4 or abs(sum(w for _,w in groups)-1)>1e-5:
            for group,_ in groups:body.vertex_groups[group].remove([vertex.index])
            total=sum(w for _,w in groups[:4])
            for group,weight in groups[:4]:body.vertex_groups[group].add([vertex.index],weight/total,'REPLACE')
    assert [k.name for k in mesh.shape_keys.key_blocks]==old_keys
    assert all(len(k.data)==len(mesh.vertices) for k in mesh.shape_keys.key_blocks)
    basis=np.array([v.co[:] for v in mesh.shape_keys.key_blocks[0].data])
    for key in list(mesh.shape_keys.key_blocks)[1:]:
        delta=np.array([v.co[:] for v in key.data])-basis
        assert np.isfinite(delta).all() and np.count_nonzero(np.linalg.norm(delta,axis=1)>1e-6)>4,(slug,key.name)
    record['vertices']=len(mesh.vertices);record['triangles']=len(mesh.polygons)
    record['cheek_refinement']=dict(original_vertices=old_vertices,vertices=len(mesh.vertices),maximum_edge_m=.012,interpolated_face_channels=23)
    if slug=='chloe':
        for side in ['L','R','closed']:
            name=f'{slug}-blink-{side}.png'
            if not (BACKUP/name).exists():shutil.copy2(ASSETS/name,BACKUP/name)
        import import_meshy_characters as painter
        painter.STAGE=ASSETS
        mat=mesh.materials[0];nodes=mat.node_tree.nodes
        base=next(n for n in nodes if n.type=='TEX_IMAGE' and n.image and n.image.name==mat['meshy_base_texture'])
        bsdf=nodes.get('Principled BSDF')
        for node in list(nodes):
            if node!=base and node.type in ['TEX_IMAGE','MIX_RGB']:nodes.remove(node)
        for image in list(bpy.data.images):
            if image.name.startswith('chloe-blink-'):bpy.data.images.remove(image)
        mat.node_tree.links.new(base.outputs['Color'],bsdf.inputs['Base Color'])
        painter.paint_blink_textures(body,face,slug)
    rig.data.pose_position='POSE';bpy.context.scene.frame_set(0)
    for key in mesh.shape_keys.key_blocks:key.value=0
    for image in bpy.data.images:
        if image.source=='FILE' and (ASSETS/Path(image.filepath).name).exists():
            image.filepath='//'+Path(image.filepath).name
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/f'{slug}.blend'),relative_remap=False)
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and o.parent==rig]
    export_fbx(body,rig,ASSETS/f'{slug}.fbx',extras)
    (ASSETS/f'{slug}-rig.json').write_text(json.dumps(record,indent=2))
    print('FACE_REFINEMENT_OK',slug,old_vertices,len(mesh.vertices),'extras',len(extras),flush=True)
