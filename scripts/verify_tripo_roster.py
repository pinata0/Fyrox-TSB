"""Validate the candidate skins, all clips, loop seams and independent FBX imports."""
import bpy
import json
import hashlib
import sys
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
import verify_tripo_legs as legs
OUT=ROOT/'artifacts/tripo-roster-import'
slugs=[s for s in ['naia','erpin','renua'] if s in sys.argv] or ['naia','erpin','renua']
candidate_dir=None
if '--candidate-dir' in sys.argv:
    assert len(slugs)==1,'--candidate-dir requires one character slug'
    candidate_dir=Path(sys.argv[sys.argv.index('--candidate-dir')+1]).resolve()
for slug in slugs:
    folder=candidate_dir if candidate_dir is not None else OUT/slug
    record=json.loads((folder/'candidate.json').read_text())
    bpy.ops.wm.open_mainfile(filepath=str(folder/f'{slug}.blend'))
    scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects[f'{slug}_SkinnedMesh']
    assert len(rig.data.bones)==record['bones']
    assert len(mesh.data.uv_layers)==1
    assert all(v.groups and abs(sum(g.weight for g in v.groups)-1)<.0001 for v in mesh.data.vertices)
    assert all(len(v.groups)<=4 for v in mesh.data.vertices)
    assert all(g.name in rig.data.bones for g in mesh.vertex_groups)
    assert all(n in rig.data.bones for n in ['Hand.R','UpperArm.R','Forearm.R'])
    for material in mesh.data.materials:
        images=[n.image for n in material.node_tree.nodes if n.type=='TEX_IMAGE' and n.image]
        assert len(images)==1 and images[0].packed_file
        assert Path(bpy.path.abspath(images[0].filepath)).is_file()
    rest=np.array([v.co[:] for v in mesh.data.vertices])
    attachment_material=f'{slug}_head-hat' if slug=='naia' else f'{slug}_hair'
    attachment_ids=set()
    for polygon in mesh.data.polygons:
        if mesh.data.materials[polygon.material_index].name==attachment_material:
            attachment_ids.update(polygon.vertices)
    if slug!='naia':attachment_ids={i for i in attachment_ids if rest[i,2]>=.70*1.85}
    assert attachment_ids,(slug,'missing head attachment')
    attachment_ids=np.array(sorted(attachment_ids))
    head_index=mesh.vertex_groups['Head'].index
    assert all(next((g.weight for g in mesh.data.vertices[int(i)].groups if g.group==head_index),0)>.99999
               for i in attachment_ids),(slug,'cap must stay fixed to Head')
    assert all(m.name!='naia_weapon-hat' for m in mesh.data.materials)
    attachment_rest=np.column_stack((rest[attachment_ids],np.ones(len(attachment_ids))))
    attachment_drift=0.0
    attachment_frames=set()
    cloth_ids={i for p in mesh.data.polygons if mesh.data.materials[p.material_index].name==f'{slug}_clothes' for i in p.vertices}
    body_ids={i for p in mesh.data.polygons if mesh.data.materials[p.material_index].name==f'{slug}_rigged' for i in p.vertices}
    leg_reports,sole_data=legs.prepare(slug,mesh,rig,rest,body_ids,cloth_ids)
    cuff_data={};cuff_reports={}
    for side,sign in [('R',1),('L',-1)]:
        wrist=np.array(rig.data.bones[f'Hand.{side}'].head_local)
        ids=np.array(sorted(i for i in cloth_ids if rest[i,0]*sign>wrist[0]*sign-.012*1.85
                            and abs(rest[i,2]-wrist[2])<.20))
        assert len(ids)>12,(slug,side,'missing cuff rim')
        rim=rest[ids];center=(rim.min(axis=0)+rim.max(axis=0))*.5
        radius=(rim.max(axis=0)-rim.min(axis=0))*.5
        radial_offset=float(np.linalg.norm((center-wrist)[1:]))
        assert radial_offset<.004,(slug,side,'cuff off the wrist axis',radial_offset)
        assert abs(float((rim[:,0]*sign).max())-wrist[0]*sign)<.005,(slug,side,'sleeve length')
        skin=np.array([rest[i] for i in body_ids if abs(rest[i,0]-wrist[0])<.008 and abs(rest[i,2]-wrist[2])<.12])
        assert len(skin)>3,(slug,side,'missing native wrist surface')
        skin_radius=np.max(np.abs(skin[:,1:]-wrist[1:]),axis=0)
        clearance=float(np.min(radius[1:]-skin_radius))
        assert clearance>.002,(slug,side,'wrist does not fit inside cuff',clearance)
        bone_name=f'Forearm.{side}';group_index=mesh.vertex_groups[bone_name].index
        assert all(next((g.weight for g in mesh.data.vertices[int(i)].groups if g.group==group_index),0)>.99999 for i in ids)
        cuff_data[side]=(ids,np.column_stack((rim,np.ones(len(ids)))),bone_name)
        cuff_reports[side]={'rim_vertices':len(ids),'radial_center_offset_m':radial_offset,
                            'wrist_clearance_m':clearance,'max_motion_drift_m':0.0}
    edges=np.array([e.vertices[:] for e in mesh.data.edges])
    lengths=np.linalg.norm(rest[edges[:,0]]-rest[edges[:,1]],axis=1)
    valid=lengths>.004
    def evaluate(frame):
        global attachment_drift
        scene.frame_set(frame);bpy.context.view_layer.update()
        evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        points=np.array([v.co[:] for v in evaluated.data.vertices])
        assert np.isfinite(points).all()
        legs.evaluate(slug,rig,points,leg_reports,sole_data,frame)
        head_transform=np.array(rig.pose.bones['Head'].matrix @ rig.data.bones['Head'].matrix_local.inverted())
        expected=(attachment_rest @ head_transform.T)[:,:3]
        drift=float(np.max(np.abs(points[attachment_ids]-expected)))
        attachment_drift=max(attachment_drift,drift)
        attachment_frames.add(frame)
        assert drift<.00001,(slug,frame,'head attachment drift',drift)
        for side,(ids,cuff_rest,bone_name) in cuff_data.items():
            transform=np.array(rig.pose.bones[bone_name].matrix @ rig.data.bones[bone_name].matrix_local.inverted())
            expected=(cuff_rest @ transform.T)[:,:3]
            cuff_drift=float(np.max(np.abs(points[ids]-expected)))
            cuff_reports[side]['max_motion_drift_m']=max(cuff_reports[side]['max_motion_drift_m'],cuff_drift)
            assert cuff_drift<.00001,(slug,side,frame,'detached cuff',cuff_drift)
        return points
    reports={}
    for name,(start,end) in record['clips'].items():
        first=evaluate(start)
        delta=float(np.max(np.abs(evaluate(start+(end-start)//4)-first)))
        assert delta>.001,(slug,name,'static')
        worst=0;fraction=0
        for frame in range(start,end+1,3):
            points=evaluate(frame)
            ratios=np.linalg.norm(points[edges[:,0]]-points[edges[:,1]],axis=1)[valid]/lengths[valid]
            worst=max(worst,float(np.quantile(ratios,.99)))
            fraction=max(fraction,float(np.mean(ratios>2)))
        assert worst<2.5,(slug,name,'skin stretching',worst)
        seam=float(np.max(np.abs(evaluate(end)-first))) if name in ['Idle','Walk','Run','Dance'] else None
        if seam is not None:assert seam<.0001,(slug,name,'loop seam',seam)
        reports[name]={'max_vertex_delta':delta,'worst_p99_edge_ratio':worst,'fraction_edges_over_2x':fraction,'loop_seam':seam}
    height=float(np.ptp(evaluate(0)[:,2]))
    # Renua's floating halo extends above her forelock; the body stays unchanged.
    assert 1.7<height<(2.8 if slug=='renua' else 2.5),height
    if slug=='renua' and record.get('head_accessory_correction'):
        accessories=sorted({i for p in mesh.data.polygons if mesh.data.materials[p.material_index].name=='renua_accessories' for i in p.vertices})
        hair=sorted({i for p in mesh.data.polygons if mesh.data.materials[p.material_index].name=='renua_hair' for i in p.vertices})
        halo=[i for i in accessories if rest[i,1]>.6]
        earphones=[i for i in accessories if rest[i,1]<0]
        assert halo and earphones
        non_halo=sorted(set(range(len(rest)))-set(halo))
        assert np.ptp(rest[non_halo,2])<2.5,'only the halo may exceed normal body height'
        assert rest[halo,1].min()>rest[hair,1].max()+.05,'halo embedded in hair'
        assert rest[halo,2].max()>rest[hair,2].max()+.15,'halo hidden behind forelock'
        assert rest[earphones,1].max()<-.30,'earphones must sit over the front side hair'
        assert rest[earphones,2].max()>2.10,'earphone fins hidden inside hair'
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(folder/f'{slug}.fbx'))
    assert any(a.frame_range[1]-a.frame_range[0]>=527 for a in bpy.data.actions)
    assert len(next(o for o in bpy.data.objects if o.type=='ARMATURE').data.bones)==record['bones']
    for img in bpy.data.images:
        if img.source=='FILE':assert Path(bpy.path.abspath(img.filepath)).is_file(),img.filepath
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(folder/f'{slug}-weapon.fbx'))
    assert any(o.type=='MESH' for o in bpy.data.objects)
    assert not any(o.type=='ARMATURE' for o in bpy.data.objects)
    weapon_materials={m.name for o in bpy.data.objects if o.type=='MESH' for m in o.data.materials}
    expected_materials={f'{slug}_weapon'}|({'naia_weapon-hat'} if slug=='naia' else set())
    assert weapon_materials==expected_materials,(slug,weapon_materials)
    for img in bpy.data.images:
        if img.source=='FILE':assert Path(bpy.path.abspath(img.filepath)).is_file(),img.filepath
    names=[f'{slug}.blend',f'{slug}.fbx',f'{slug}-weapon.fbx','candidate.json']+[Path(p).name for p in record['textures']]
    result={'passed':True,'bones':record['bones'],'height':height,'clips':reports,'fbx_roundtrip':True,
            'head_attachment_vertices':len(attachment_ids),'head_attachment_max_drift':attachment_drift,
            'head_attachment_frames_checked':len(attachment_frames),
            'sleeve_cuffs':cuff_reports,
            'leg_fitting':leg_reports,
            'weapon_materials':sorted(weapon_materials),
            'candidate_sha256':{name:hashlib.sha256((folder/name).read_bytes()).hexdigest() for name in names}}
    (folder/'verification.json').write_text(json.dumps(result,indent=2))
    print('ROSTER_VERIFY_OK',slug,flush=True)
