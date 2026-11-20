"""Apply delivered props/companions while preserving animated part names and pivots."""
import bpy
import itertools
import json
import sys
from pathlib import Path
import numpy as np
from mathutils.kdtree import KDTree
from mathutils import Matrix
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from lowpoly_transfer import *
from match_lowpoly_assets import fit
from export_fyrox_fbx import export_fbx
WORK=ROOT/'artifacts/lowpoly-import';BACK=WORK/'before';STAGE=WORK/'characters'
entries=json.loads((ROOT/'assets/character-sources/lowpoly/manifest.json').read_text())['entries']

def alignment(source,target):
    """Resolve the source FBX axis conventions, then refine uniform registration."""
    tree=KDTree(len(target))
    for i,p in enumerate(target):tree.insert(p,i)
    tree.balance()
    sample=source[np.linspace(0,len(source)-1,min(1600,len(source))).astype(int)]
    candidates=[]
    for permutation in itertools.permutations(range(3)):
        for signs in itertools.product([-1,1],repeat=3):
            rotation=np.eye(3)[list(permutation)]*np.array(signs)[:,None]
            if np.linalg.det(rotation)<0:continue
            p=sample@rotation.T
            scale=max(np.ptp(target,axis=0))/max(np.ptp(p,axis=0))
            offset=(target.min(0)+target.max(0))/2-(p.min(0)+p.max(0))/2*scale
            distance=np.array([tree.find(q)[2] for q in p*scale+offset])
            candidates.append((float(np.quantile(distance,.65)),rotation))
    results=[]
    for _,rotation in sorted(candidates,key=lambda r:r[0])[:3]:
        result=fit(source@rotation.T,target)
        result['rotation']=(np.array(result['rotation'])@rotation).tolist()
        results.append(result)
    result=min(results,key=lambda r:r['median'])
    assert result['median']<.025,('alignment failed',result)
    return result

def combined(objects):
    vertices=[];faces=[];uvs=[];owners=[]
    for owner,obj in enumerate(objects):
        obj.data.calc_loop_triangles();offset=len(vertices)
        vertices.extend([obj.matrix_world@v.co for v in obj.data.vertices])
        for t in obj.data.loop_triangles:
            faces.append([i+offset for i in t.vertices]);owners.append(owner)
            uvs.extend([obj.data.uv_layers.active.data[i].uv[:] for i in t.loops])
    data=bpy.data.meshes.new('Transfer surface');data.from_pydata(vertices,[],faces);data.update()
    layer=data.uv_layers.new();layer.data.foreach_set('uv',np.array(uvs).ravel())
    proxy=bpy.data.objects.new('Transfer surface',data);bpy.context.collection.objects.link(proxy)
    return proxy,np.array(owners)

def material(new,path):
    original=new.data.materials[0]
    bsdf=original.node_tree.nodes.get('Principled BSDF')
    image=bsdf.inputs['Base Color'].links[0].from_node.image
    image.scale(1024,1024);image.filepath_raw=str(path);image.file_format='PNG';image.save()
    image.pack()
    mat=bpy.data.materials.new(path.stem);mat.use_nodes=True
    shader=mat.node_tree.nodes.get('Principled BSDF');shader.inputs['Roughness'].default_value=.85
    node=mat.node_tree.nodes.new('ShaderNodeTexImage');node.image=image
    mat.node_tree.links.new(node.outputs['Color'],shader.inputs['Base Color'])
    return mat

def replace_parts(entry,objects,budget,image_name):
    new=append_mesh(ROOT/entry['source']);reduce_mesh(new,budget)
    proxy,owners=combined(objects);target=Surface(proxy)
    alignment_target=target.p.copy()
    sebastian=len(objects)==1 and objects[0].name=='Sebastian'
    def head_shift(p):
        smooth=lambda v: np.clip(v,0,1)**2*(3-2*np.clip(v,0,1))
        return .40*np.maximum(smooth((p[:,2]-1.48)/.24),smooth((np.abs(p[:,0])-.65)/.15)*smooth((p[:,2]-1.10)/.18))
    if sebastian:
        # The carried bear has an authored forward head offset; fit its
        # original source shape, then apply the same deformation to the remesh.
        alignment_target[:,1]+=head_shift(alignment_target)
    registration=alignment(points(new),alignment_target)
    p=transformed(new,registration)
    if sebastian:
        p[:,1]-=head_shift(p)
        new.data.vertices.foreach_set('co',p.astype(np.float32).ravel())
        registration['authored_head_shift']=.40
    centers=np.array([p[list(f.vertices)].mean(0) for f in new.data.polygons])
    assignments=owners[target.nearest(centers)[3]]
    mat=material(new,STAGE/image_name)
    details=[]
    for owner,old in enumerate(objects):
        polygons=[f for f in new.data.polygons if assignments[f.index]==owner]
        assert polygons,(old.name,'empty replacement part')
        used=sorted({i for f in polygons for i in f.vertices});index={v:i for i,v in enumerate(used)}
        inverse=np.array(old.matrix_world.inverted())
        local=np.column_stack([p[used],np.ones(len(used))])@inverse.T
        data=bpy.data.meshes.new(old.name+'_Lowpoly');data.from_pydata(local[:,:3],[],[[index[i] for i in f.vertices] for f in polygons]);data.update()
        uv=data.uv_layers.new();coords=[new.data.uv_layers.active.data[i].uv[:] for f in polygons for i in f.loop_indices];uv.data.foreach_set('uv',np.array(coords).ravel())
        part=bpy.data.objects.new('Lowpoly transfer',data);bpy.context.collection.objects.link(part)
        original=Surface(old);transfer_skin_and_shapes(old,part,original)
        names=[g.name for g in part.vertex_groups];rows=[[(g.group,g.weight) for g in v.groups] for v in part.data.vertices]
        action=old.data.shape_keys.animation_data if old.data.shape_keys else None
        if action and action.action:
            part.data.shape_keys.animation_data_create();part.data.shape_keys.animation_data.action=action.action
            part.data.shape_keys.animation_data.action_slot=action.action_slot
        old.data=part.data;old.vertex_groups.clear()
        for name in names:old.vertex_groups.new(name=name)
        for i,row in enumerate(rows):
            for g,w in row:old.vertex_groups[g].add([i],w,'REPLACE')
        old.data.materials.clear();old.data.materials.append(mat)
        for f in old.data.polygons:f.material_index=0;f.use_smooth=True
        bpy.data.objects.remove(part,do_unlink=True)
        details.append(dict(name=old.name,triangles=len(old.data.polygons)))
    bpy.data.objects.remove(proxy,do_unlink=True);bpy.data.objects.remove(new,do_unlink=True)
    return dict(source=entry['source'],parts=details,alignment=registration)

def export_prop(slug,kind):
    entry=next(r for r in entries if r['slug']==slug and r['kind']==kind)
    bpy.ops.wm.open_mainfile(filepath=str(BACK/f'{slug}-{kind}.blend'))
    scene=bpy.context.scene;scene.frame_set(0)
    for image in bpy.data.images:
        name=Path(bpy.path.abspath(image.filepath)).name
        if (STAGE/name).is_file():image.filepath=str(STAGE/name)
    objects=[bpy.data.objects[f'{slug}_{kind}']]
    if slug=='ui':objects.append(bpy.data.objects['Ui_Clover'])
    report=replace_parts(entry,objects,3000 if kind!='mount' else 6000,f'{slug}-{kind}-meshy-basecolor.png')
    # The axle is a separate authored green cylinder with the old atlas.
    if slug=='ui':
        import shutil
        path=STAGE/'ui-weapon-detail-basecolor.png';shutil.copy2(BACK/'ui-weapon-meshy-basecolor.png',path)
        for mat in bpy.data.objects['Ui_Clover_Axle'].data.materials:
            for node in mat.node_tree.nodes:
                if node.type=='TEX_IMAGE' and node.image:node.image.filepath=str(path)
    bpy.context.preferences.filepaths.save_version=0;bpy.ops.outliner.orphans_purge(do_recursive=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}-{kind}.blend'),relative_remap=False)
    all_meshes=[o for o in scene.objects if o.type=='MESH']
    for obj in all_meshes:
        if obj.data.shape_keys:
            obj.data.shape_keys.animation_data_clear()
            for k in obj.data.shape_keys.key_blocks:k.value=0
        if obj.name.startswith('Aya_Blade_'):obj.animation_data_clear();obj.scale=(1,1,1)
    bpy.ops.object.select_all(action='DESELECT')
    for obj in all_meshes:obj.select_set(True)
    bpy.context.view_layer.objects.active=objects[0]
    bpy.ops.export_scene.fbx(filepath=str(STAGE/f'{slug}-{kind}.fbx'),use_selection=True,object_types={'MESH'},axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
    path=STAGE/f'{slug}-{kind}-prop.json';meta=json.loads(path.read_text())
    meta['triangles']=sum(sum(len(p.vertices)-2 for p in o.data.polygons) for o in all_meshes)
    meta['vertices']=sum(len(o.data.vertices) for o in all_meshes);meta['lowpoly']=report;path.write_text(json.dumps(meta,indent=2))
    (WORK/f'{slug}-{kind}-report.json').write_text(json.dumps(report,indent=2))
    print('LOWPOLY_PROP_OK',slug,kind,meta['triangles'],flush=True)

def companions(slug):
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    rig=bpy.data.objects['TSB_Rig'];rig.data.pose_position='REST';bpy.context.scene.frame_set(0);bpy.context.view_layer.update()
    kinds=['robot','mount'] if slug=='id' else ['companion']
    reports=[]
    for kind in kinds:
        entry=next(r for r in entries if r['slug']==slug and r['kind']==kind)
        names=([o.name for o in bpy.context.scene.objects if o.name.startswith('IdPodMesh.')] if kind=='mount' else
               ['Nata'] if kind=='robot' else ['Frog'] if slug=='ui' else ['Sebastian'])
        reports.append(replace_parts(entry,[bpy.data.objects[n] for n in names],6000 if kind=='mount' else 5000 if kind=='companion' else 3000,names[0].split('.')[0]+'-lowpoly-basecolor.png'))
    rig.data.pose_position='POSE';bpy.context.scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version=0;bpy.ops.outliner.orphans_purge(do_recursive=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'),relative_remap=False)
    export_fbx(bpy.data.objects[slug+'_SkinnedMesh'],rig,STAGE/f'{slug}.fbx')
    (WORK/f'{slug}-companions-report.json').write_text(json.dumps(reports,indent=2))
    print('LOWPOLY_COMPANIONS_OK',slug,flush=True)

if __name__=='__main__':
    args=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
    if args and args[0]=='companions':
        for slug in args[1:]:companions(slug)
    else:
        for entry in entries:
            if entry['kind'] in ['weapon','mount','robot'] and (not args or entry['slug'] in args):export_prop(entry['slug'],entry['kind'])
