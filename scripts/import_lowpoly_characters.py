"""Stage delivered low-poly bodies on the production rigs without changing motion."""
import bpy
import json
import shutil
import sys
from pathlib import Path
import numpy as np

ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from lowpoly_transfer import *
from export_fyrox_fbx import export_fbx
WORK=ROOT/'artifacts/lowpoly-import';BACK=WORK/'before';STAGE=WORK/'characters'
manifest=json.loads((ROOT/'assets/character-sources/lowpoly/manifest.json').read_text())['entries']
matches={r['file']:r for r in json.loads((WORK/'matches.json').read_text())}

def run(slug):
    entry=next(r for r in manifest if r['slug']==slug and r['kind']=='character')
    match=matches[entry['original']];assert match['ranked'][0]==slug,match['ranked']
    fit=match['matches'][slug];assert fit['median']<.007,fit
    bpy.ops.wm.open_mainfile(filepath=str(BACK/f'{slug}.blend'))
    scene=bpy.context.scene;scene.frame_set(0);rig=bpy.data.objects['TSB_Rig'];rig.data.pose_position='REST'
    body=bpy.data.objects[slug+'_SkinnedMesh'];surface=Surface(body)
    before=sum(len(o.data.polygons) for o in scene.objects if o.type=='MESH')
    for image in bpy.data.images:
        filename=Path(bpy.path.abspath(image.filepath)).name
        if (STAGE/filename).is_file():image.filepath=str(STAGE/filename)
    new=append_mesh(ROOT/entry['source']);transformed(new,fit)
    removed=trim_unused_arms(new,body,surface)
    indices,w,distance=transfer_skin_and_shapes(body,new,surface)
    print('LOWPOLY_SKIN',slug,len(new.data.vertices),len(new.data.polygons),'removed arms',removed,flush=True)
    projection=uv_projection(new,surface)
    textures=[BACK/f'{slug}-meshy-basecolor.png',*BACK.glob(f'{slug}-blink-*.png')]
    if slug=='naia':textures+=list(BACK.glob('naia-attack-*.png'))
    textures+=list((BACK/'cheeks').glob(f'{slug}-*.png'))
    base=None
    for source in textures:
        target=STAGE/source.relative_to(BACK);target.parent.mkdir(parents=True,exist_ok=True)
        image=reproject_image(source,target,*projection)
        if source.name==f'{slug}-meshy-basecolor.png':base=image
        else:bpy.data.images.remove(image)
    assert base is not None
    new.data.materials.clear()
    mat=bpy.data.materials.new(slug+'_meshy_lowpoly');mat.use_nodes=True;mat['meshy_base_texture']=base.name
    bsdf=mat.node_tree.nodes.get('Principled BSDF');bsdf.inputs['Roughness'].default_value=.85
    tex=mat.node_tree.nodes.new('ShaderNodeTexImage');tex.image=base;mat.node_tree.links.new(tex.outputs['Color'],bsdf.inputs['Base Color'])
    new.data.materials.append(mat)
    for polygon in new.data.polygons:polygon.material_index=0;polygon.use_smooth=True
    source=json.loads((BACK/f'{slug}-cheeks.json').read_text())
    by_position={tuple(round(v*10000) for v in row[:3]):row[3:] for row in source['vertices']}
    controls=np.array([by_position.get(tuple(round(v*10000) for v in p),[0]*8+[8]) for p in surface.p])
    control=interpolate(controls,indices,w)
    # Never interpolate a protected eye/accessory vertex into an active cheek.
    protected=np.max(np.where(w>1e-5,controls[indices,8],0),axis=1)>0
    control[protected,:8]=0;control[:,8]=np.where(protected,8,0)
    source['vertices']=np.column_stack([points(new),control]).tolist()
    source['lowpoly_transfer']='surface-barycentric-v1'
    (STAGE/f'{slug}-cheeks.json').write_text(json.dumps(source,separators=(',',':')))
    newgroups=[g.name for g in new.vertex_groups]
    rows=[[(g.group,g.weight) for g in v.groups] for v in new.data.vertices]
    body.data=new.data;body.vertex_groups.clear()
    for name in newgroups:body.vertex_groups.new(name=name)
    for i,row in enumerate(rows):
        for group,weight in row:body.vertex_groups[group].add([i],weight,'REPLACE')
    bpy.data.objects.remove(new,do_unlink=True)
    hand_report={}
    for obj in list(scene.objects):
        if obj.type!='MESH':continue
        if '_RefinedHand.' in obj.name:hand_report[obj.name]=reduce_mesh(obj,1500)
        elif obj.name.endswith('_RestoredCoat'):hand_report[obj.name]=reduce_mesh(obj,1600)
    rig.data.pose_position='POSE';scene.frame_set(0)
    for key in body.data.shape_keys.key_blocks:key.value=0
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.outliner.orphans_purge(do_recursive=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'),relative_remap=False)
    export_fbx(body,rig,STAGE/f'{slug}.fbx')
    record=json.loads((BACK/f'{slug}-rig.json').read_text())
    record['triangles']=len(body.data.polygons)
    record['lowpoly']=dict(source=entry['source'],body_triangles=len(body.data.polygons),hands=hand_report,
        rig_and_actions_preserved=True,alignment=fit,removed_arm_triangles=removed,
        surface_distance_p99=float(np.quantile(distance,.99)))
    (STAGE/f'{slug}-rig.json').write_text(json.dumps(record,indent=2))
    report=dict(slug=slug,before_triangles=before,after_triangles=sum(len(o.data.polygons) for o in scene.objects if o.type=='MESH'),**record['lowpoly'])
    (WORK/f'{slug}-report.json').write_text(json.dumps(report,indent=2))
    print('LOWPOLY_CHARACTER_OK',slug,report['before_triangles'],report['after_triangles'],flush=True)

if __name__=='__main__':
    for subdir in ['cheeks']:
        if not (BACK/subdir).exists():shutil.copytree(ROOT/'assets/characters'/subdir,BACK/subdir)
        if not (STAGE/subdir).exists():shutil.copytree(BACK/subdir,STAGE/subdir)
    for slug in sys.argv[sys.argv.index('--')+1:]:run(slug)
