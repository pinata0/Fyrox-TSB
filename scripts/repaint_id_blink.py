"""Refresh only Id's blink atlases/materials while preserving the current rig.

Run in Blender, inspect the candidate, then repeat with -- --publish.
"""
import bpy,hashlib,json,shutil,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'));sys.dont_write_bytecode=True
from export_fyrox_fbx import resolve_character_textures
import import_meshy_characters as painter
ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/id-blink-shadow-fix'
BEFORE=OUT/'before';DEST=OUT/'candidate'
DEST.mkdir(parents=True,exist_ok=True);BEFORE.mkdir(exist_ok=True)
FILES=['id.blend']+[f'id-blink-{stage}.png' for stage in ['L','R','half','closed']]
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()

if '--publish' in sys.argv:
    verified=json.loads((OUT/'verification.json').read_text())
    for name in FILES:
        assert digest(ASSETS/name)==digest(BEFORE/name),(name,'source changed during review')
        assert digest(DEST/name)==verified['sha256'][name],(name,'candidate changed')
    bpy.ops.wm.open_mainfile(filepath=str(DEST/'id.blend'))
    for name in FILES[1:]:shutil.copy2(DEST/name,ASSETS/name)
    for image in bpy.data.images:
        name=Path(bpy.path.abspath(image.filepath)).name
        if (ASSETS/name).is_file():image.filepath='//'+name
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'id.blend'),relative_remap=False)
    path=ASSETS/'manifest.json';manifest=json.loads(path.read_text(encoding='utf-8'))
    record=next(r for r in manifest if r['slug']=='id')
    for key,name in [('blend','id.blend'),('closed_eyes','id-blink-closed.png'),('half_closed_eyes','id-blink-half.png')]:
        record['sha256'][key]=digest(ASSETS/name)
    record['white_lash_blink']['open_lid_shadow_erased']=True
    path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    (OUT/'published.json').write_text(json.dumps({name:digest(ASSETS/name) for name in FILES},indent=2))
    print('ID_BLINK_SHADOW_PUBLISHED',flush=True)
    raise SystemExit(0)

for name in FILES:
    if not (BEFORE/name).exists():shutil.copy2(ASSETS/name,BEFORE/name)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'id.blend'))
body=bpy.data.objects['id_SkinnedMesh']
resolve_character_textures([o for o in bpy.context.scene.objects if o.type=='MESH'])
for image in bpy.data.images:
    if image.source=='FILE':
        path=ASSETS/Path(bpy.path.abspath(image.filepath)).name
        if path.is_file():image.filepath=str(path)
face=json.loads((ASSETS/'id-rig.json').read_text(encoding='utf-8'))['face']
shapes={k.name:[v.co[:] for v in k.data] for k in body.data.shape_keys.key_blocks}
painter.STAGE=DEST
painter.paint_blink_textures(body,face,'id',build_nodes=False,natural=True)
assert shapes=={k.name:[v.co[:] for v in k.data] for k in body.data.shape_keys.key_blocks}
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(DEST/'id.blend'),relative_remap=False)
report=dict(all_shape_keys_preserved=True,sha256={name:digest(DEST/name) for name in FILES})
(OUT/'verification.json').write_text(json.dumps(report,indent=2))
print('ID_BLINK_SHADOW_CANDIDATE_OK',flush=True)
