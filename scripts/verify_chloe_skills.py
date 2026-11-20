"""Check fixed arm lengths on every authored frame and between baked keys."""
import bpy,json
from pathlib import Path
P=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(P/'assets/characters/chloe.blend'))
rig=bpy.data.objects['TSB_Rig'];scene=bpy.context.scene;scene.frame_set(0)
def lengths(side):
    return [(rig.pose.bones[f'{b}.{side}'].head-rig.pose.bones[f'{a}.{side}'].head).length for a,b in [('UpperArm','Forearm'),('Forearm','Hand')]]
reference={s:lengths(s) for s in ['L','R']};largest=0.0;count=0
for frame in list(range(63,121))+list(range(531,838)):
    for sub in [0,.5]:
        scene.frame_set(frame,subframe=sub)
        for side in ['L','R']:
            for actual,expected in zip(lengths(side),reference[side]):
                error=abs(actual-expected);largest=max(largest,error)
                assert error<.002,(frame,sub,side,actual,expected)
        count+=1
for side in ['L','R']:
    for suffix in ['RefinedSleeve','RefinedViewSleeve']:
        obj=bpy.data.objects[f'chloe_{suffix}.{side}']
        assert all(len(v.groups)==1 and abs(v.groups[0].weight-1)<1e-6 for v in obj.data.vertices),'sleeve spans segments'
result={'passed':True,'sampled_poses':count,'fixed_lengths':reference,'maximum_length_error':largest,'sleeves':'rigid upper arm / forearm sections with rounded elbow'}
(P/'artifacts/chloe-skills/verification.json').write_text(json.dumps(result,indent=2))
print('CHLOE_FIXED_ARMS_OK',json.dumps(result),flush=True)
