"""Visible closed geometry, finger contact, texture and face-preservation checks."""
import bpy,bmesh,json,sys
import numpy as np
from pathlib import Path
from mathutils import Quaternion,Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'artifacts/hand-mesh-polish';DEST=OUT/'characters'
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else 'vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
reports=[]
for slug in selected:
    bpy.ops.wm.open_mainfile(filepath=str(OUT/'before'/f'{slug}.blend'))
    source=bpy.data.objects[f'{slug}_SkinnedMesh']
    shapes={k.name:np.array([v.co[:] for v in k.data]) for k in source.data.shape_keys.key_blocks}
    affected=set()
    for name,points in shapes.items():
        if name!='Basis':affected.update(int(i) for i in np.flatnonzero(np.linalg.norm(points-shapes['Basis'],axis=1)>1e-7))
    face_polygons={tuple(p.vertices) for p in source.data.polygons if any(v in affected for v in p.vertices)}
    bpy.ops.wm.open_mainfile(filepath=str(DEST/f'{slug}.blend'))
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
    assert len(body.data.shape_keys.key_blocks)==24
    for k in body.data.shape_keys.key_blocks:
        assert np.max(np.abs(np.array([v.co[:] for v in k.data])-shapes[k.name]))<1e-7,(slug,k.name,'face changed')
    assert face_polygons.issubset({tuple(p.vertices) for p in body.data.polygons}),(slug,'facial polygons removed')
    meshes=[o for o in bpy.data.objects if o.type=='MESH' and '_Refined' in o.name]
    for obj in meshes:
        bm=bmesh.new();bm.from_mesh(obj.data)
        boundaries=sum(e.is_boundary for e in bm.edges);nonmanifold=sum(not e.is_manifold for e in bm.edges)
        assert boundaries==nonmanifold==0,(slug,obj.name,boundaries,nonmanifold)
        assert all(f.calc_area()>1e-10 for f in bm.faces),(slug,obj.name,'degenerate face')
        bm.free()
        assert all(0<len(v.groups)<=4 and abs(sum(g.weight for g in v.groups)-1)<1e-4 for v in obj.data.vertices)
        assert obj.data.uv_layers.active and len(obj.data.uv_layers.active.data)==len(obj.data.loops)
        for m in obj.data.materials:
            assert m.get('meshy_base_texture'),(slug,obj.name,'texture bake missing')
            assert any(n.type=='TEX_IMAGE' and n.image and Path(bpy.path.abspath(n.image.filepath)).is_file() for n in m.node_tree.nodes)
    rig.animation_data_clear();rig.data.pose_position='POSE'
    for bone in rig.pose.bones:
        bone.rotation_mode='QUATERNION';bone.rotation_quaternion=Quaternion();bone.location=(0,0,0);bone.scale=(1,1,1)
    bpy.context.view_layer.update()
    profiles=json.loads((DEST/f'{slug}-hands.json').read_text())
    hand_reports=[]
    for p in profiles:
        side=p['side'];obj=bpy.data.objects[f'{slug}_RefinedHand.{side}'];dg=bpy.context.evaluated_depsgraph_get()
        open_points=np.array([v.co[:] for v in obj.evaluated_get(dg).data.vertices])
        assert (rig.data.bones[f'Thumb3.{side}'].tail_local-rig.data.bones[f'Index3.{side}'].tail_local).length>.04,(slug,side,'thumb is fused with index')
        for c in p['controls']:
            bone=rig.pose.bones[c['name']]
            axis=rig.pose.bones[p['hand']].matrix.to_3x3()@Vector(c['axis'])
            before=bone.matrix.copy();rotation=Quaternion(axis,c['angle']).to_matrix()@before.to_3x3()
            result=rotation.to_4x4();result.translation=before.translation;bone.matrix=result;bpy.context.view_layer.update()
        bpy.context.view_layer.update();posed=np.array([v.co[:] for v in obj.evaluated_get(dg).data.vertices]);assert np.isfinite(posed).all()
        edges=np.array([e.vertices[:] for e in obj.data.edges]);rest_length=np.linalg.norm(open_points[edges[:,0]]-open_points[edges[:,1]],axis=1)
        ratio=np.linalg.norm(posed[edges[:,0]]-posed[edges[:,1]],axis=1)/np.maximum(rest_length,1e-7)
        delta=posed-np.array(p['contact']);radial=np.sqrt((delta@np.array(p['forward']))**2+(delta@np.array(p['dorsal']))**2)
        digits=[]
        for digit in ['Index','Middle','Ring','Pinky','Thumb']:
            indices={g.index for g in obj.vertex_groups if g.name.startswith(digit)}
            mask=np.array([sum(g.weight for g in v.groups if g.group in indices)>.35 for v in obj.data.vertices])
            assert mask.sum()>15,(slug,side,digit,'missing visible weights')
            movement=float(np.quantile(np.linalg.norm(posed-open_points,axis=1)[mask],.9));assert movement>.01,(slug,side,digit,'did not articulate')
            gap=float(np.min(np.abs(radial[mask]-p['radius'])));assert gap<.003,(slug,side,digit,'not touching grip',gap)
            assert np.quantile(np.maximum(p['radius']-radial[mask],0),.99)<.002,(slug,side,digit,'penetrates handle')
            digits.append(dict(digit=digit,vertices=int(mask.sum()),movement=movement,grip_gap=gap,penetration_p99=float(np.quantile(np.maximum(p['radius']-radial[mask],0),.99))))
        p99=float(np.quantile(ratio,.99));assert p99<3.0,(slug,side,'skin stretch',p99)
        hand_reports.append(dict(side=side,closed_surface=True,p99_edge_ratio=p99,digits=digits,
            handle_penetration_p99=float(np.quantile(np.maximum(p['radius']-radial,0),.99))))
    reports.append(dict(slug=slug,passed=True,face_morphs=23,face_polygons_preserved=len(face_polygons),closed_meshes=len(meshes),hands=hand_reports))
    (OUT/'mesh-verification.json').write_text(json.dumps(dict(passed=len(reports)==15,characters=reports),indent=2))
    print('REFINED_HAND_VERIFY_OK',slug,hand_reports,flush=True)
