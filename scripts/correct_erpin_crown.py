"""Create a crown-only correction from the backed-up delivered Erpin model.

Run in Blender background. Review artifacts/erpin-crown-correction/candidate
before publishing. The same fitting function is used by import_tripo_roster.
"""
import bpy
import hashlib
import json
import shutil
import sys
import numpy as np
from pathlib import Path
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from import_tripo_roster import fit_erpin_crown, render
from export_fyrox_fbx import export_fbx

WORK=ROOT/'artifacts/erpin-crown-correction'
OUT=WORK/'candidate'
BEFORE=WORK/'before/assets/characters'
OUT.mkdir(exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'erpin.blend'))
mesh=bpy.data.objects['erpin_SkinnedMesh']
rig=bpy.data.objects['TSB_Rig']
scene=bpy.context.scene
scene.frame_set(0)
rest=np.array([v.co[:] for v in mesh.data.vertices])
weights=[[(g.group,g.weight) for g in v.groups] for v in mesh.data.vertices]
accessories={i for p in mesh.data.polygons if mesh.data.materials[p.material_index].name=='erpin_accessories' for i in p.vertices}
head=mesh.vertex_groups['Head'].index
crown=sorted(i for i in accessories if any(g.group==head and g.weight>.99999 for g in mesh.data.vertices[i].groups))
assert len(crown)==1108,('unexpected crown vertices',len(crown))
camera=scene.camera
scene.cycles.samples=24
scene.render.resolution_x=scene.render.resolution_y=640
camera.data.ortho_scale=1.30
render(scene,camera,WORK/'before-front.png',0,(0,-5,1.65),(0,0,1.65))
render(scene,camera,WORK/'before-side.png',0,(5,0,1.65),(0,0,1.65))
fit_erpin_crown(mesh,crown,unit_scale=1.85)
mesh.data.update()
after=np.array([v.co[:] for v in mesh.data.vertices])
other=sorted(set(range(len(rest)))-set(crown))
assert np.array_equal(rest[other],after[other])
assert weights==[[(g.group,g.weight) for g in v.groups] for v in mesh.data.vertices]
assert np.allclose(np.ptp(after[crown],axis=0)/np.ptp(rest[crown],axis=0),(.62,.62,.52),atol=1e-6)
attachment=np.column_stack((after[crown],np.ones(len(crown))))
max_drift=0.0
for frame in range(529):
    scene.frame_set(frame)
    bpy.context.view_layer.update()
    evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    points=np.array([evaluated.data.vertices[i].co[:] for i in crown])
    transform=np.array(rig.pose.bones['Head'].matrix @ rig.data.bones['Head'].matrix_local.inverted())
    drift=float(np.max(np.abs(points-(attachment @ transform.T)[:,:3])))
    max_drift=max(max_drift,drift)
    assert drift<1e-5,(frame,drift)
scene.frame_set(0)
for image in bpy.data.images:
    if image.name.startswith('erpin-') and image.source=='FILE':
        name=Path(image.filepath).name
        source=ROOT/'assets/characters'/name
        assert source.is_file(),source
        shutil.copy2(source,OUT/name)
        image.filepath=str(OUT/name)
        if not image.packed_file:image.pack()
shutil.copy2(ROOT/'assets/characters/erpin-weapon.fbx',OUT/'erpin-weapon.fbx')
scene['erpin_crown_correction']='Illustration proportion: width/depth 62%, height 52%; seated on scalp; 2026-11-13'
camera.data.ortho_scale=2.6
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'erpin.blend'))
export_fbx(mesh,rig,OUT/'erpin.fbx')
render(scene,camera,OUT/'erpin-preview.png',0,(2.8,-5,2.2))
camera.data.ortho_scale=1.30
render(scene,camera,WORK/'after-front.png',0,(0,-5,1.65),(0,0,1.65))
render(scene,camera,WORK/'after-side.png',0,(5,0,1.65),(0,0,1.65))
render(scene,camera,WORK/'after-back.png',0,(0,5,1.65),(0,0,1.65))
camera.data.ortho_scale=1.22
scene.render.resolution_x=scene.render.resolution_y=256
render(scene,camera,OUT/'erpin-portrait.png',0,(0,-5,1.50),(0,0,1.50))
record=next(r for r in json.loads((ROOT/'assets/characters/manifest.json').read_text(encoding='utf-8')) if r['slug']=='erpin')
record['crown_correction']={'scale_xyz':[.62,.62,.52],'rise_m':.0555,'reference':'assets/character-inputs/inputs-masked/erpin.png'}
(OUT/'candidate.json').write_text(json.dumps(record,ensure_ascii=False,indent=2),encoding='utf-8')
report={'passed':True,'crown_vertices':len(crown),'unchanged_other_vertices':len(other),'weights_unchanged':True,
        'crown_size_before_m':np.ptp(rest[crown],axis=0).tolist(),'crown_size_after_m':np.ptp(after[crown],axis=0).tolist(),
        'head_attachment_frames':529,'head_attachment_max_drift_m':max_drift,
        'candidate_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in OUT.iterdir()
                            if p.is_file() and (p.suffix in {'.blend','.fbx','.jpg','.png'} or p.name=='candidate.json')}}
(WORK/'correction-verification.json').write_text(json.dumps(report,indent=2))
print('ERPIN_CROWN_CORRECTION_OK',json.dumps(report),flush=True)
