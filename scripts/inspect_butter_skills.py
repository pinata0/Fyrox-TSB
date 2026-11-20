import bpy,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
out={}
for kind in ['butter','butter-weapon']:
 bpy.ops.wm.open_mainfile(filepath=str(ROOT/f'assets/characters/{kind}.blend'));bpy.context.scene.frame_set(0)
 data=[]
 for o in bpy.context.scene.objects:
  if o.type=='ARMATURE':data.append({'rig':o.name,'bones':{b.name:[list(b.head),list(b.tail)] for b in o.pose.bones}})
  if o.type=='MESH':
   groups={g.name:sum(vg.weight for v in o.data.vertices for vg in v.groups if vg.group==g.index) for g in o.vertex_groups if g.name in ['Tail','Hips','Torso']}
   data.append({'mesh':o.name,'bounds':[[min(v.co[j] for v in o.data.vertices),max(v.co[j] for v in o.data.vertices)] for j in range(3)],'groups':groups,'shape_keys':[k.name for k in o.data.shape_keys.key_blocks] if o.data.shape_keys else [],'matrix':[list(row) for row in o.matrix_world]})
 out[kind]=data
(ROOT/'artifacts/butter-skills/inspection.json').write_text(json.dumps(out,indent=2))
