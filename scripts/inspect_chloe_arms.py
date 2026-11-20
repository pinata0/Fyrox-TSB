import bpy,json
from pathlib import Path
P=Path(__file__).resolve().parents[1]
bpy.ops.wm.open_mainfile(filepath=str(P/'assets/characters/chloe.blend'))
r=bpy.data.objects['TSB_Rig'];out=[]
for f in [0,63,71,531,539,558,576,588,599,618,621,634,664,669,687,714,720,740,760,777,799,837]:
 bpy.context.scene.frame_set(f)
 out.append({'frame':f,'arms':{s:{'lengths':[(r.pose.bones[f'{b}.{s}'].head-r.pose.bones[f'{a}.{s}'].head).length for a,b in [('UpperArm','Forearm'),('Forearm','Hand')]],'scales':{n:list(r.pose.bones[n+'.'+s].scale) for n in ['UpperArm','Forearm','Hand']},'locations':{n:list(r.pose.bones[n+'.'+s].location) for n in ['UpperArm','Forearm','Hand']}} for s in ['L','R']}})
(P/'artifacts/chloe-skills/arm-lengths-before.json').write_text(json.dumps(out,indent=2))
print('ARM_LENGTHS',[(o['frame'],o['arms']['L']['lengths'],o['arms']['L']['scales']) for o in out])
