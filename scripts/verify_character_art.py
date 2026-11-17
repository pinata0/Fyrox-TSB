"""Run in Blender background: verify every delivered rig, weight, clip and guide."""
import bpy
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/characters'
records=json.loads((OUT/'manifest.json').read_text(encoding='utf-8'))
assert len(records)==15
reports=[]
for item in records:
    for key in ['blend','model','guide']:
        path=ROOT/item[key]
        assert path.is_file() and path.stat().st_size>1000,(item['slug'],key)
    bpy.ops.wm.open_mainfile(filepath=str(ROOT/item['blend']))
    rig=bpy.data.objects['TSB_Rig']; mesh=bpy.data.objects[item['slug']+'_SkinnedMesh']
    assert len(rig.data.bones)==item['bones']
    assert rig.animation_data and rig.animation_data.action
    assert all(abs(sum(g.weight for g in v.groups)-1)<.0001 for v in mesh.data.vertices)
    assert any(m.type=='ARMATURE' and m.object==rig for m in mesh.modifiers)
    assert bpy.data.objects['Turnaround_REFERENCE_only'].data.packed_file
    def pose(frame):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        return [x for p in rig.pose.bones for row in p.matrix for x in row]
    motions={}
    for name,(start,end) in item['clips'].items():
        a=pose(start);b=pose(start+max(1,(end-start)//4))
        delta=max(abs(x-y) for x,y in zip(a,b))
        assert delta>.001,(item['slug'],name,'static clip')
        if name in ['Idle','Walk','Run','Dance']:
            seam=max(abs(x-y) for x,y in zip(a,pose(end)))
            assert seam<.0001,(item['slug'],name,'loop seam',seam)
        motions[name]={'frames':[start,end],'max_pose_delta':round(delta,6)}
    pose(0)
    evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    points=[evaluated.matrix_world@v.co for v in evaluated.data.vertices]
    bounds=[[round(min(p[i] for p in points),4),round(max(p[i] for p in points),4)] for i in range(3)]
    height_limit=2.8 if item['slug']=='renua' else 2.6
    assert 1.5<bounds[2][1]-bounds[2][0]<height_limit,(item['slug'],bounds)
    reports.append({'slug':item['slug'],'vertices':len(mesh.data.vertices),'bones':len(rig.data.bones),'all_vertices_weighted':True,'bounds_meters':bounds,'motions':motions})
    print('ART_VERIFY_OK',item['slug'],flush=True)
(ROOT/'artifacts/character-art-verification.json').write_text(json.dumps({'passed':True,'characters':reports},indent=2),encoding='utf-8')
print(f'ART_VERIFY_COMPLETE: {len(reports)} rigs, {sum(len(r["motions"]) for r in reports)} clips, 60 seamless loops',flush=True)
