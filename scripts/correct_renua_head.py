"""Correct Renua's delivered head attachments without rebaking her motions.

Run in Blender background. Outputs a candidate plus geometry/motion checks under
artifacts/renua-head-correction; use --publish after inspecting the renders.
"""
import bpy
import hashlib
import json
import shutil
import sys
import numpy as np
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from import_tripo_roster import fit_renua_head_accessories, render
from export_fyrox_fbx import export_fbx

WORK=ROOT/'artifacts/renua-head-correction'
OUT=WORK/'candidate'
ASSETS=ROOT/'assets/characters'
OUT.mkdir(parents=True,exist_ok=True)
if '--publish' in sys.argv:
    checked=json.loads((OUT/'verification.json').read_text())
    assert checked['passed']
    for name,digest in checked['candidate_sha256'].items():
        assert hashlib.sha256((OUT/name).read_bytes()).hexdigest()==digest,name
    record=json.loads((OUT/'candidate.json').read_text(encoding='utf-8'))
    for name in ['renua.fbx','renua-preview.png','renua-portrait.png']:
        shutil.copy2(OUT/name,ASSETS/name)
    bpy.ops.wm.open_mainfile(filepath=str(OUT/'renua.blend'))
    for img in bpy.data.images:
        if img.name.startswith('renua-') and img.source=='FILE':img.filepath='//'+Path(img.filepath).name
    bpy.context.scene.frame_set(0)
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'renua.blend'))
    records=json.loads((ASSETS/'manifest.json').read_text(encoding='utf-8'))
    next(r for r in records if r['slug']=='renua').update(record)
    (ASSETS/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2),encoding='utf-8')
    print('RENUA_HEAD_PUBLISHED',flush=True)
    sys.exit(0)
bpy.ops.wm.open_mainfile(filepath=str(WORK/'before/assets/characters/renua.blend'))
mesh=bpy.data.objects['renua_SkinnedMesh']
rig=bpy.data.objects['TSB_Rig']
scene=bpy.context.scene
scene.frame_set(0)
rest=np.array([v.co[:] for v in mesh.data.vertices])
weights=[[(g.group,g.weight) for g in v.groups] for v in mesh.data.vertices]
accessories=sorted({i for p in mesh.data.polygons if mesh.data.materials[p.material_index].name=='renua_accessories' for i in p.vertices})
head=mesh.vertex_groups['Head'].index
assert all(any(g.group==head and g.weight>.99999 for g in mesh.data.vertices[i].groups) for i in accessories)
regions=fit_renua_head_accessories(mesh,accessories,unit_scale=1.85)
assert all(regions.values())
mesh.data.update()
after=np.array([v.co[:] for v in mesh.data.vertices])
other=sorted(set(range(len(rest)))-set(accessories))
assert np.array_equal(rest[other],after[other])
assert weights==[[(g.group,g.weight) for g in v.groups] for v in mesh.data.vertices]
hair=sorted({i for p in mesh.data.polygons if mesh.data.materials[p.material_index].name=='renua_hair' for i in p.vertices})
halo=after[regions['halo']]
assert halo[:,1].min()-after[hair,1].max()>.05,'halo intersects the back of the hair'
assert halo[:,2].max()-after[hair,2].max()>.15,'halo must rise above the forelock'
attachment=np.column_stack((after[accessories],np.ones(len(accessories))))
max_drift=0.0
for frame in range(529):
    scene.frame_set(frame)
    bpy.context.view_layer.update()
    evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    points=np.array([evaluated.data.vertices[i].co[:] for i in accessories])
    transform=np.array(rig.pose.bones['Head'].matrix @ rig.data.bones['Head'].matrix_local.inverted())
    drift=float(np.max(np.abs(points-(attachment @ transform.T)[:,:3])))
    max_drift=max(max_drift,drift)
    assert drift<1e-5,(frame,drift)
scene.frame_set(0)
record=next(r for r in json.loads((ASSETS/'manifest.json').read_text(encoding='utf-8')) if r['slug']=='renua')
for path in record['textures']:
    shutil.copy2(ROOT/path,OUT/Path(path).name)
for img in bpy.data.images:
    if img.name.startswith('renua-') and img.source=='FILE':
        img.filepath=str(OUT/Path(img.filepath).name)
        if not img.packed_file:img.pack()
shutil.copy2(ASSETS/'renua-weapon.fbx',OUT/'renua-weapon.fbx')
scene['renua_head_correction']='Earphones outside hair; halo raised behind head; 2026-11-14'
scene.camera.data.ortho_scale=3.1
scene.cycles.samples=24
scene.render.resolution_x=scene.render.resolution_y=800
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'renua.blend'))
export_fbx(mesh,rig,OUT/'renua.fbx')
render(scene,scene.camera,OUT/'renua-preview.png',0,(2.8,-5,2.3),(0,0,1.35))
scene.camera.data.ortho_scale=2.3
for name,pos in [('front',(0,-5,1.90)),('side',(5,0,1.90)),('back',(0,5,1.90))]:
    render(scene,scene.camera,WORK/f'after-{name}.png',0,pos,(0,0,1.90))
scene.camera.data.ortho_scale=1.75
scene.render.resolution_x=scene.render.resolution_y=256
render(scene,scene.camera,OUT/'renua-portrait.png',0,(0,-5,1.90),(0,0,1.90))
record['head_accessory_correction']={'reference':'assets/character-inputs/inputs-masked/renua.png',
    'halo_hair_depth_clearance_m':float(halo[:,1].min()-after[hair,1].max()),
    'halo_above_forelock_m':float(halo[:,2].max()-after[hair,2].max()),
    'region_vertices':{k:len(v) for k,v in regions.items()}}
(OUT/'candidate.json').write_text(json.dumps(record,ensure_ascii=False,indent=2),encoding='utf-8')
report={'passed':True,'unchanged_other_vertices':len(other),'weights_unchanged':True,
        'attachment_frames':529,'max_attachment_drift_m':max_drift,
        **record['head_accessory_correction'],
        'candidate_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in OUT.iterdir() if p.is_file() and p.suffix in {'.blend','.fbx','.png','.jpg'}}}
(WORK/'correction-verification.json').write_text(json.dumps(report,indent=2))
print('RENUA_HEAD_CORRECTION_OK',json.dumps(report),flush=True)
