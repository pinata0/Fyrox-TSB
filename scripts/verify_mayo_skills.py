"""Verify delivered timeline, sampled keys, geometry preservation and mesh groups."""
import bpy,json,hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];ASSETS=ROOT/'assets/characters';WORK=ROOT/'artifacts/mayo-skills'
bpy.ops.wm.open_mainfile(filepath=str(ASSETS/'mayo.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig']
assert scene.frame_end==694
record=json.loads((ASSETS/'mayo-rig.json').read_text(encoding='utf-8'))
assert record['clips']['MayoSkill']==[531,591] and record['clips']['MayoUltimate']==[594,694]
source=json.loads((ASSETS/'skill/mayo.json').read_text())
assert source['sha256']==hashlib.sha256((ROOT/'assets/references/mayo-motion-reference.skel').read_bytes()).hexdigest()
preserved=json.loads((WORK/'bake-verification.json').read_text())
assert preserved['geometry_preserved'] and preserved['old_keys_preserved']
samples=[]
for start,end in [(531,591),(594,694)]:
 for f in range(start,end+1):
  scene.frame_set(f)
  for p in rig.pose.bones:
   assert all(abs(v)<100 for row in p.matrix for v in row),f'nonfinite bone {p.name} frame {f}'
  if f-start in [0,12,24,32,49,65,77]:samples.append({'frame':f,'feet':[list(rig.pose.bones[n].head) for n in ['Foot.L','Foot.R']],'hips':list(rig.pose.bones['Hips'].head)})
groups=json.loads((ROOT/'assets/effects/mayo-guns.json').read_text());names={g['group'] for g in groups}
assert names=={'skill','ultimate','rotor'}|{f'belt{i}' for i in range(8)}
assert all(max(max(t) for t in g['triangles'])<len(g['vertices']) for g in groups)
(WORK/'asset-verification.json').write_text(json.dumps({'source_hash_matches':True,'preserved_existing_assets':True,'frames_checked':162,'triangles':sum(len(g['triangles']) for g in groups),'samples':samples},indent=2))
print('MAYO_ASSETS_OK',flush=True)
