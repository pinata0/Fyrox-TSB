"""Publish the verified three-character candidates; originals remain under character-sources."""
import bpy
import json
import hashlib
import shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
WORK=ROOT/'artifacts/tripo-roster-import'
OUT=ROOT/'assets/characters'
records=json.loads((OUT/'manifest.json').read_text(encoding='utf-8'))
for slug in ['naia','erpin','renua']:
    folder=WORK/slug
    checked=json.loads((folder/'verification.json').read_text())
    assert checked['passed']
    for name,digest in checked['candidate_sha256'].items():
        assert hashlib.sha256((folder/name).read_bytes()).hexdigest()==digest,(slug,name)
    candidate=json.loads((folder/'candidate.json').read_text())
    for name in [f'{slug}.fbx',f'{slug}-weapon.fbx',f'{slug}-portrait.png',f'{slug}-preview.png']+[Path(p).name for p in candidate['textures']]:
        shutil.copy2(folder/name,OUT/name)
    bpy.ops.wm.open_mainfile(filepath=str(folder/f'{slug}.blend'))
    for kind in ['rigged','hair','clothes','cape','wings','accessories','bracelet','sandals','head-hat','weapon-hat','weapon']:
        image=bpy.data.images.get(f'{slug}-{kind}-basecolor')
        if image:image.filepath=f'//{slug}-{kind}-basecolor.jpg'
    bpy.context.scene.frame_set(0)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/f'{slug}.blend'))
    next(r for r in records if r['slug']==slug).update(candidate)
    print('ROSTER_PUBLISHED',slug,flush=True)
(OUT/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2),encoding='utf-8')
unchanged=json.loads((WORK/'before/other-model-hashes.json').read_text())
for name,digest in unchanged.items():assert hashlib.sha256((OUT/name).read_bytes()).hexdigest()==digest,name
(WORK/'publication.json').write_text(json.dumps({'published':['naia','erpin','renua'],
    'unchanged_other_models':len(unchanged),'source_archives':19,'source_directory':'assets/character-sources/tripo'},indent=2))
