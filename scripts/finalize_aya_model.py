"""Render Aya and validate ankle-length twin tails, held parasol and baked rig."""
import bpy
import json
import sys
from pathlib import Path
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
import build_characters as author

bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/aya.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects['aya_SkinnedMesh']
scene['selection_gesture']='parasol greeting'
scene['aya_correction']='Two ankle-length low white twin tails weighted to Hair; held flower parasol weighted to Hand.L.'
scene.frame_set(0)
assert bpy.data.objects['Turnaround_REFERENCE_only'].data.packed_file
assert len(rig.data.bones)==20
assert all(abs(sum(g.weight for g in vert.groups)-1)<.0001 for vert in mesh.data.vertices)
hand_index=mesh.vertex_groups['Hand.L'].index
held=[v for v in mesh.data.vertices if any(g.group==hand_index and g.weight>.99 for g in v.groups)]
assert len(held)>500
assert max((mesh.matrix_world@v.co).z for v in held)>2.0
hair_index=mesh.vertex_groups['Hair'].index
hair=[mesh.matrix_world@v.co for v in mesh.data.vertices if any(g.group==hair_index and g.weight>.99 for g in v.groups)]
tails={}
for sign,label in [(-1,'left'),(1,'right')]:
    vertices=[v for v in hair if sign*v.x>0]
    assert len(vertices)>100
    lower=min(v.z for v in vertices);upper=max(v.z for v in vertices)
    assert .08<lower<.20,(label,'must reach ankle',lower)
    assert upper>1.23,(label,'must connect to low tie',upper)
    assert min(sign*v.x for v in vertices if v.z<1.1)>.30,(label,'separate tail silhouette')
    tails[label]={'vertices':len(vertices),'min_z':lower,'max_z':upper}

def pose(frame):
    scene.frame_set(frame);bpy.context.view_layer.update()
    return [x for p in rig.pose.bones for row in p.matrix for x in row]

motions={}
for name,(start,end) in author.CLIPS.items():
    a=pose(start);b=pose(start+max(1,(end-start)//4))
    delta=max(abs(x-y) for x,y in zip(a,b));assert delta>.001,(name,'static')
    seam=None
    if name in ['Idle','Walk','Run','Dance']:
        seam=max(abs(x-y) for x,y in zip(a,pose(end)));assert seam<.0001,(name,seam)
    motions[name]={'delta':delta,'loop_seam':seam}
scene.frame_set(0)
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'assets/characters/aya.blend'))
scene.camera.location=(0,-5,1.60)
scene.camera.rotation_euler=(Vector((-.06,0,1.60))-scene.camera.location).to_track_quat('-Z','Y').to_euler()
scene.camera.data.ortho_scale=1.85
scene.render.resolution_x=256;scene.render.resolution_y=256;scene.cycles.samples=16
scene.render.filepath=str(ROOT/'assets/characters/aya-portrait.png')
bpy.ops.render.render(write_still=True)
for label,location in [('front',(0,-5,1.4)),('back',(0,5,1.4))]:
    scene.camera.location=location
    scene.camera.rotation_euler=(Vector((0,0,1.1))-scene.camera.location).to_track_quat('-Z','Y').to_euler()
    scene.camera.data.ortho_scale=2.95;scene.render.resolution_x=512;scene.render.resolution_y=512
    scene.render.filepath=str(ROOT/f'artifacts/aya-twintail-correction/model-{label}.png')
    bpy.ops.render.render(write_still=True)
progress=json.loads((ROOT/'assets/characters/build-progress.json').read_text())[0]
records=json.loads((ROOT/'assets/characters/manifest.json').read_text(encoding='utf-8'))
for item in records:
    if item['slug']=='aya':
        item.update(progress)
        item.update(selection_gesture='parasol greeting',portrait='assets/characters/aya-portrait.png',parasol_in_model=True,correction='Held flower parasol; two ankle-length low white twin tails; 2026-11-12')
(ROOT/'assets/characters/manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2),encoding='utf-8')
(ROOT/'artifacts/aya-twintail-correction/model-validation.json').write_text(json.dumps({'passed':True,'bones':20,'clips':motions,'hand_weighted_vertices':len(held),'vertices':len(mesh.data.vertices),'ankle_length_twintails':tails},indent=2),encoding='utf-8')
print('AYA_TWINTAIL_MODEL_VERIFIED',len(mesh.data.vertices),tails,flush=True)
