"""Check weighted fingers, distinct palm contacts and actual closed-hand deformation."""
import bpy,json,sys
import numpy as np
from pathlib import Path
from mathutils import Vector,Quaternion
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'artifacts/meshy-hand-fix';DEST=ROOT/'assets/characters'
reports=[]
for path in sorted(DEST.glob('*-hands.json')):
    slug=path.name.removesuffix('-hands.json');profiles=json.loads(path.read_text())
    bpy.ops.wm.open_mainfile(filepath=str(DEST/f'{slug}.blend'))
    rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects[f'{slug}_SkinnedMesh'];bpy.context.scene.frame_set(0)
    rig.animation_data_clear();bpy.context.view_layer.update()
    original=np.array([v.co[:] for v in body.evaluated_get(bpy.context.evaluated_depsgraph_get()).data.vertices])
    hands=[]
    for p in profiles:
        assert len(p['controls'])==15
        weighted={c['name']:sum(any(body.vertex_groups[g.group].name==c['name'] and g.weight>.001 for g in v.groups) for v in body.data.vertices) for c in p['controls']}
        # Distal fingertip bones can be unweighted on fused mittens; every digit must deform.
        assert all(sum(weighted[f'{d}{i}.{p["side"]}'] for i in [1,2,3])>4 for d in ['Thumb','Index','Middle','Ring','Pinky']),(slug,weighted)
        for c in p['controls']:
            bone=rig.pose.bones[c['name']]
            axis=bone.bone.matrix_local.to_3x3().inverted()@(rig.data.bones[p['hand']].matrix_local.to_3x3()@Vector(c['axis']))
            bone.rotation_quaternion=Quaternion(axis,c['angle'])
        hands.append(dict(side=p['side'],weighted_vertices=weighted,palm_distance=float(np.linalg.norm(np.array(p['contact'])-p['wrist']))))
        assert hands[-1]['palm_distance']>.015
    bpy.context.view_layer.update();evaluated=body.evaluated_get(bpy.context.evaluated_depsgraph_get())
    posed=np.array([v.co[:] for v in evaluated.data.vertices]);assert np.isfinite(posed).all()
    for side in ['L','R']:
        finger_groups={body.vertex_groups[c['name']].index for p in profiles if p['side']==side for c in p['controls']}
        affected=np.array([any(g.group in finger_groups and g.weight>.1 for g in v.groups) for v in body.data.vertices])
        movement=np.linalg.norm(posed-original,axis=1)
        assert np.quantile(movement[affected],.9)>.015,(slug,side,'hand stayed open')
        for digit in ['Thumb','Index','Middle','Ring','Pinky']:
            groups={body.vertex_groups[f'{digit}{i}.{side}'].index for i in [1,2,3]}
            mask=np.array([any(g.group in groups and g.weight>.01 for g in v.groups) for v in body.data.vertices])
            assert np.max(movement[mask])>.005,(slug,side,digit,'weighted digit did not bend')
    # The face geometry and 23 facial shapes must survive the hand-only change.
    assert len(body.data.shape_keys.key_blocks)==24
    reports.append(dict(slug=slug,hands=hands,finite=True,closed_deformation=True,face_morphs=23))
    print('ARTICULATED_HANDS_OK',slug,flush=True)
(OUT/'hand-verification.json').write_text(json.dumps(dict(passed=len(reports)==15,characters=reports),indent=2))
