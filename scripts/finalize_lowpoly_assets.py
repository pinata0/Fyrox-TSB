"""Finalize portable staged assets and validate rig/animation preservation."""
import bpy
import hashlib
import json
import shutil
import sys
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
WORK=ROOT/'artifacts/lowpoly-import';STAGE=WORK/'characters';BACK=WORK/'before'
SLUGS='vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()

def rig_signature(rig):
    bones=[(b.name,b.parent.name if b.parent else None,[list(row) for row in b.matrix_local],list(b.head_local),list(b.tail_local)) for b in rig.data.bones]
    action=rig.animation_data.action
    curves=action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
    keys=[(c.data_path,c.array_index,[(tuple(k.co),k.interpolation,tuple(k.handle_left),tuple(k.handle_right)) for k in c.keyframe_points]) for c in curves]
    return hashlib.sha256(json.dumps([bones,keys]).encode()).hexdigest()

def referenced_images(meshes):
    return {n.image for o in meshes for m in o.data.materials if m and m.use_nodes for n in m.node_tree.nodes if n.type=='TEX_IMAGE' and n.image}

records=json.loads((STAGE/'manifest.json').read_text());by_slug={r['slug']:r for r in records};report=[]
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
for path in sorted(STAGE.glob('*.blend')):
    if selected and path.stem not in selected:continue
    slug=path.stem;is_character=slug in SLUGS
    if is_character:
        bpy.ops.wm.open_mainfile(filepath=str(BACK/path.name));signature=rig_signature(bpy.data.objects['TSB_Rig'])
    bpy.ops.wm.open_mainfile(filepath=str(path));scene=bpy.context.scene;scene.frame_set(0)
    meshes=[o for o in scene.objects if o.type=='MESH']
    if is_character:
        rig=bpy.data.objects['TSB_Rig'];assert rig_signature(rig)==signature,(slug,'rig or animation changed')
        body=bpy.data.objects[slug+'_SkinnedMesh']
        # Small authored eye/hair patches retain their old atlas coordinates.
        for image in referenced_images([o for o in meshes if o!=body]):
            name=Path(bpy.path.abspath(image.filepath)).name
            if name==f'{slug}-meshy-basecolor.png' or name.startswith(f'{slug}-blink-'):
                target=STAGE/(slug+'-detail-'+name[len(slug)+1:]);shutil.copy2(BACK/name,target)
                image.filepath=str(target)
        for name in ['blink_normal_L','blink_normal_R']:
            attr=body.data.attributes.get(name)
            if attr:
                normal=np.array([v.vector[:] for v in attr.data]);normal/=np.maximum(np.linalg.norm(normal,axis=1,keepdims=True),1e-9)
                attr.data.foreach_set('vector',normal.astype(np.float32).ravel())
        assert len(body.data.shape_keys.key_blocks)==24,slug
        for key in body.data.shape_keys.key_blocks:key.value=0
        for obj in meshes:
            for v in obj.data.vertices:
                assert np.isfinite(v.co).all(),(obj.name,'nonfinite geometry')
                weights=[g.weight for g in v.groups if g.weight>1e-6]
                if obj.vertex_groups:assert len(weights)<=4 and abs(sum(weights)-1)<1e-4,(obj.name,v.index,weights)
        rig.data.pose_position='POSE'
        stats=[]
        for obj in meshes:
            obj.data.calc_loop_triangles();stats.append(dict(name=obj.name,vertices=len(obj.data.vertices),triangles=len(obj.data.loop_triangles)))
        record=json.loads((STAGE/f'{slug}-rig.json').read_text())
        record['triangles']=len(body.data.loop_triangles);record['vertices']=len(body.data.vertices)
        record['lowpoly']['meshes']=stats;record['lowpoly']['total_triangles']=sum(s['triangles'] for s in stats)
        for hand in record.get('refined_hands',[]):
            obj=bpy.data.objects[slug+'_RefinedHand.'+hand['side']]
            hand['hand_vertices']=len(obj.data.vertices);hand['hand_faces']=len(obj.data.polygons)
        (STAGE/f'{slug}-rig.json').write_text(json.dumps(record,indent=2))
        by_slug[slug].update(vertices=len(body.data.vertices),triangles=len(body.data.loop_triangles),lowpoly=record['lowpoly'],
            source='User supplied low-poly Meshy delivery, 2026-11-20; existing TSB rig and animation retained')
        report.append(dict(slug=slug,bones=len(rig.data.bones),rig_animation_sha256=signature,meshes=stats))
    for image in referenced_images(meshes):
        name=Path(bpy.path.abspath(image.filepath)).name
        assert (STAGE/name).is_file(),(slug,image.name,image.filepath)
        image.filepath='//'+name
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(path),relative_remap=False)
    if is_character:export_fbx(body,rig,path.with_suffix('.fbx'),texture_path_mode='STRIP')
    else:
        for obj in meshes:
            if obj.data.shape_keys:
                obj.data.shape_keys.animation_data_clear()
                for key in obj.data.shape_keys.key_blocks:key.value=0
            if obj.name.startswith('Aya_Blade_'):obj.animation_data_clear();obj.scale=(1,1,1)
        bpy.ops.object.select_all(action='DESELECT')
        for obj in meshes:obj.select_set(True)
        bpy.context.view_layer.objects.active=meshes[0]
        bpy.ops.export_scene.fbx(filepath=str(path.with_suffix('.fbx')),use_selection=True,object_types={'MESH'},axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='STRIP')
    (WORK/'rig-validation.json').write_text(json.dumps(report,indent=2))
    print('LOWPOLY_VALIDATED',slug,flush=True)
(STAGE/'manifest.json').write_text(json.dumps(records,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
