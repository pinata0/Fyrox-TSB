"""Build Id's white-lash blink correction from a preserved source snapshot.

Blender --background --python-exit-code 1 --python scripts/fix_id_blink.py
Candidate assets are written under artifacts/id-blink-fix/candidate.
"""
import bpy, hashlib, json, shutil, sys, runpy
import numpy as np
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'));sys.dont_write_bytecode=True
from id_eyelids import rebuild_white_lashes
from export_fyrox_fbx import export_fbx,resolve_character_textures
import import_meshy_characters as painter
ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/id-blink-fix'
DEST=OUT/'candidate';BEFORE=OUT/'before'
DEST.mkdir(parents=True,exist_ok=True);BEFORE.mkdir(exist_ok=True)
FILES=['id'+s for s in ['.blend','.fbx','.morphs.json','-rig.json','-blink-L.png','-blink-R.png','-blink-half.png','-blink-closed.png']]
if '--publish' in sys.argv:
    def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
    verified=json.loads((OUT/'verification.json').read_text())
    for name in FILES:
        assert digest(ASSETS/name)==digest(BEFORE/name),(name,'source changed during review')
        assert digest(DEST/name)==verified['candidate_sha256'][name],(name,'candidate changed after verification')
    bpy.ops.wm.open_mainfile(filepath=str(DEST/'id.blend'))
    for name in FILES:
        if name!='id.blend':shutil.copy2(DEST/name,ASSETS/name)
    for image in bpy.data.images:
        name=Path(bpy.path.abspath(image.filepath)).name
        if (ASSETS/name).is_file():image.filepath='//'+name
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'id.blend'),relative_remap=False)
    export_fbx(bpy.data.objects['id_SkinnedMesh'],bpy.data.objects['TSB_Rig'],ASSETS/'id.fbx')
    sys.argv=['export_meshy_morphs.py','--',str(ASSETS),'id']
    runpy.run_path(str(ROOT/'scripts/export_meshy_morphs.py'),run_name='__main__')
    assert json.loads((ASSETS/'id.morphs.json').read_text())==json.loads((DEST/'id.morphs.json').read_text())
    path=ASSETS/'manifest.json';manifest=json.loads(path.read_text(encoding='utf-8'))
    record=next(r for r in manifest if r['slug']=='id')
    for key,name in [('blend','id.blend'),('model','id.fbx'),('morph_data','id.morphs.json'),
                     ('closed_eyes','id-blink-closed.png'),('half_closed_eyes','id-blink-half.png')]:
        record['sha256'][key]=digest(ASSETS/name)
    record['white_lash_blink']=json.loads((ASSETS/'id-rig.json').read_text(encoding='utf-8'))['white_lash_blink']
    path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    (OUT/'published.json').write_text(json.dumps({name:digest(ASSETS/name) for name in FILES},indent=2))
    print('ID_WHITE_LASH_PUBLISHED',flush=True)
    raise SystemExit(0)
for name in FILES:
    if not (BEFORE/name).exists():shutil.copy2(ASSETS/name,BEFORE/name)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'id.blend'))
body=bpy.data.objects['id_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
resolve_character_textures([o for o in bpy.context.scene.objects if o.type=='MESH'])
for image in bpy.data.images:
    if image.source=='FILE':
        path=ASSETS/Path(bpy.path.abspath(image.filepath)).name
        if path.is_file():image.filepath=str(path)
record=json.loads((BEFORE/'id-rig.json').read_text())
stats=rebuild_white_lashes(body,record['face'])
print('ID_WHITE_LASH_GEOMETRY',stats,flush=True)
painter.STAGE=DEST
if '--geometry-only' not in sys.argv:
    painter.paint_blink_textures(body,record['face'],'id',build_nodes=False,natural=True)
else:
    for material in body.data.materials:
        for node in material.node_tree.nodes:
            if node.type=='TEX_IMAGE' and node.image and node.image.name.startswith('id-blink-'):
                path=DEST/(node.image.name.split('.')[0]+'.png')
                assert path.is_file(),path
                node.image.filepath=str(path);node.image.reload();node.image.pack()
bpy.context.scene.frame_set(0)
for key in body.data.shape_keys.key_blocks:key.value=0
record['white_lash_blink']=dict(revision=1,geometry=stats,painted_dark_lash=False)
basis=np.array([v.co[:] for v in body.data.vertices])
for key in body.data.shape_keys.key_blocks:
    if key.name.startswith(('eyeBlink_','browDown_')):
        delta=np.array([v.co[:] for v in key.data])-basis
        record['face']['morph_vertices'][key.name]=int((np.linalg.norm(delta,axis=1)>1e-6).sum())
(DEST/'id-rig.json').write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(DEST/'id.blend'),relative_remap=False)
export_fbx(body,rig,DEST/'id.fbx')
sys.argv=['export_meshy_morphs.py','--',str(DEST),'id']
runpy.run_path(str(ROOT/'scripts/export_meshy_morphs.py'),run_name='__main__')
print('ID_WHITE_LASH_CANDIDATE_OK',flush=True)
