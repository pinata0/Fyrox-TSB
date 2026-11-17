"""Independent staged Blender/FBX validation before publishing runtime assets."""
import bpy
import json
import sys
import os
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from import_meshy_characters import STAGE, OUT, SLUGS
from arm_weight_isolation import arm_bones
STAGE=Path(os.environ.get('TSB_CHARACTER_ASSET_DIR',STAGE))
reports=[]
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS
for slug in selected:
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects[f'{slug}_SkinnedMesh'];scene=bpy.context.scene
    record=json.loads((STAGE/f'{slug}-rig.json').read_text())
    assert len(rig.data.bones)==record['bones']
    assert len(body.data.shape_keys.key_blocks)==24
    assert all(k.value==0 for k in body.data.shape_keys.key_blocks)
    if record.get('arm_weight_isolation'):
        excluded=arm_bones(rig)
        assert all(not any(body.vertex_groups[g.group].name in excluded and g.weight>0
                           for g in vertex.groups) for vertex in body.data.vertices), (slug,'body inherited arm weights')
    meshes=[o for o in bpy.context.scene.objects if o.type=='MESH']
    for mesh in meshes:
        assert all(0<len(v.groups)<=4 and abs(sum(g.weight for g in v.groups)-1)<1e-4 for v in mesh.data.vertices),(slug,mesh.name,'weights')
        assert all(g.name in rig.data.bones for g in mesh.vertex_groups)
        assert any(m.type=='ARMATURE' and m.object==rig for m in mesh.modifiers)
        for m in mesh.data.materials:
            for n in m.node_tree.nodes:
                if n.type=='TEX_IMAGE' and n.image:assert Path(bpy.path.abspath(n.image.filepath)).is_file()
    rest=np.array([v.co[:] for v in body.data.vertices]);edges=np.array([e.vertices[:] for e in body.data.edges])
    lengths=np.linalg.norm(rest[edges[:,0]]-rest[edges[:,1]],axis=1);valid=lengths>.006
    reference_edges=None
    reference=ROOT/f'artifacts/cheek-isolation/before/{slug}.blend'
    if record.get('cheek_refinement') and reference.is_file():
        # Compare animation strain on the same edges before/after refinement.
        # New facial samples otherwise change the percentile population.
        with bpy.data.libraries.load(str(reference),link=False) as (available,loaded):
            loaded.objects=[f'{slug}_SkinnedMesh']
        original=loaded.objects[0].data
        reference_rest=np.array([v.co[:] for v in original.vertices])
        assert np.array_equal(rest[:len(reference_rest)],reference_rest),(slug,'original surface changed')
        reference_edges=np.array([e.vertices[:] for e in original.edges])
        reference_lengths=np.linalg.norm(reference_rest[reference_edges[:,0]]-reference_rest[reference_edges[:,1]],axis=1)
        reference_valid=reference_lengths>.006
    def pose(frame):
        scene.frame_set(frame);bpy.context.view_layer.update()
        m=body.evaluated_get(bpy.context.evaluated_depsgraph_get())
        p=np.empty(len(m.data.vertices)*3,dtype=np.float32);m.data.vertices.foreach_get('co',p);p=p.reshape(-1,3)
        assert np.isfinite(p).all();return p
    clips={}
    for name,(start,end) in record['clips'].items():
        first=pose(start);last=pose(end)
        seam=float(np.max(np.abs(first-last))) if name in ['Idle','Walk','Run','Dance'] else None
        if seam is not None:assert seam<.0002,(slug,name,'loop seam',seam)
        middle=pose(start+(end-start)//3);movement=float(np.max(np.abs(first-middle)))
        assert movement>.0001,(slug,name,'static motion')
        ratios=np.linalg.norm(middle[edges[:,0]]-middle[edges[:,1]],axis=1)[valid]/lengths[valid]
        refined_p99=float(np.quantile(ratios,.99))
        if reference_edges is not None:
            reference_ratios=np.linalg.norm(middle[reference_edges[:,0]]-middle[reference_edges[:,1]],axis=1)[reference_valid]/reference_lengths[reference_valid]
            assert float(ratios.max())<=float(reference_ratios.max())*1.20+.02,(slug,name,'refinement introduced extreme strain')
            p99=float(np.quantile(reference_ratios,.99))
        else:p99=refined_p99
        assert p99<3.2,(slug,name,'skin stretching',p99)
        clips[name]=dict(movement=movement,loop_seam=seam,p99_edge_ratio=p99,refined_p99_edge_ratio=refined_p99,floor=float(middle[:,2].min()))
    # Cheek morphs cannot move any visible eye points, including the rim.
    face=record['face'];eye=(np.abs(np.abs(rest[:,0]-face['center_x'])-face['eye_x'])<.08)&(np.abs(rest[:,2]-face['eye_y'])<face['eye_half_height']*.85)&(rest[:,1]<-face['front']+.02)
    for side in ['L','R']:
        d=np.array([p.co[:] for p in body.data.shape_keys.key_blocks[f'cheekStretch_{side}'].data])-rest
        assert np.max(np.linalg.norm(d[eye],axis=1),initial=0)<.004,(slug,'eye affected by cheek')
    bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.fbx(filepath=str(STAGE/f'{slug}.fbx'))
    imported=next(o for o in bpy.data.objects if o.type=='MESH' and o.name.startswith(f'{slug}_SkinnedMesh'))
    assert len(imported.data.shape_keys.key_blocks)==24,(slug,'FBX lost morphs')
    assert any(a.frame_range[1]-a.frame_range[0]>=527 for a in bpy.data.actions),(slug,'FBX lost motion')
    assert len(next(o for o in bpy.data.objects if o.type=='ARMATURE').data.bones)==record['bones']
    for image in bpy.data.images:
        if image.source=='FILE':assert Path(bpy.path.abspath(image.filepath)).is_file(),image.filepath
    reports.append(dict(slug=slug,passed=True,body_vertices=len(rest),bones=record['bones'],morphs=23,clips=clips,fbx_roundtrip=True))
    (OUT/'verification.json').write_text(json.dumps(dict(passed=len(reports)==15,characters=reports),indent=2))
    print('MESHY_VERIFY_OK',slug,flush=True)
