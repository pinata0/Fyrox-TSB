"""Verify baked clips, localized tail weights and preserved pre-existing character data."""
import bpy,json,math
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'artifacts/butter-skills'
def read(path):
 bpy.ops.wm.open_mainfile(filepath=str(path));rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['butter_SkinnedMesh']
 curves=rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
 return ({(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves},
         [v.co[:] for v in body.data.vertices],
         [{body.vertex_groups[g.group].name:g.weight for g in v.groups if g.weight>1e-6} for v in body.data.vertices])
old=read(OUT/'before/butter.blend');new=read(ROOT/'assets/characters/butter.blend')
assert all(new[0][k]==v for k,v in old[0].items())
assert old[1]==new[1]
changed=[i for i,(a,b) in enumerate(zip(old[2],new[2])) if a!=b]
assert len(changed)==1357
assert all(new[2][i].get('Tail',0)>0 for i in changed)
assert all(abs(sum(new[2][i].values())-1)<.002 for i in changed)
assert all(len(new[2][i])<=4 for i in changed),'GPU influence limit'
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];points=[]
for frame in [531,540,549,551,556,576,579,600,612,649,652,686,721,727,732,770]:
 scene.frame_set(frame);bpy.context.view_layer.update()
 pose={k:list(rig.pose.bones[k].head) for k in ['Hips','Head','Hand.L','Hand.R']}
 assert all(math.isfinite(v) for values in pose.values() for v in values)
 points.append({'frame':frame,'pose':pose})
assert points[2]['pose']['Hips'][1]>points[4]['pose']['Hips'][1]+.10,'charge must lean back, then move forward'
(OUT/'asset-verification.json').write_text(json.dumps({'old_keys_preserved':True,'geometry_preserved':True,'localized_tail_vertices':len(changed),'four_weight_limit':True,'checkpoints':points},indent=2))
print('BUTTER_ASSET_VERIFICATION_OK',len(changed))
