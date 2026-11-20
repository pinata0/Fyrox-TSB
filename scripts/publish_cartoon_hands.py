"""Publish the independently verified hand stage without reverting other work."""
import hashlib
import json
import shutil
import sys
from pathlib import Path
import bpy

sys.path.insert(0,str(Path(__file__).resolve().parent))
from cartoon_hands import OUT, STAGE, BACK, ASSETS, ROOT, SLUGS
from export_fyrox_fbx import export_fbx


def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()


report=json.loads((OUT/'verification.json').read_text())
assert report['passed'] and {r['slug'] for r in report['characters']}==set(SLUGS)
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS
prepared=OUT/'published';prepared.mkdir(exist_ok=True)
manifest=json.loads((ASSETS/'manifest.json').read_text(encoding='utf-8'))
if selected==['--finalize']:
    for row in manifest:
        slug=row['slug']
        if slug not in SLUGS:continue
        details=json.loads((ASSETS/f'{slug}-rig.json').read_text())
        row['bones']=details['bones'];row['cartoon_hands']={k:v for k,v in details['cartoon_hands'].items() if k not in ('untouched_meshes','grip_fit')}
        row['cartoon_hands_texture']=f'assets/characters/{slug}-cartoon-hands.png'
        for key in ('blend','model','hands','cartoon_hands_texture'):
            row['sha256'][key]=digest(ROOT/row[key])
    (ASSETS/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('CARTOON_HANDS_MANIFEST_OK',flush=True)
    sys.exit(0)
# Check the entire publication up front, before changing any live file.
for slug in selected:
    for suffix in ('.blend','.fbx','-hands.json','-rig.json','.morphs.json'):
        current=digest(ASSETS/f'{slug}{suffix}')
        previous=prepared/f'{slug}{suffix}'
        assert current==digest(BACK/f'{slug}{suffix}') or (previous.exists() and current==digest(previous)),(slug,suffix,'asset changed during review')
for row in manifest:
    slug=row['slug']
    if slug not in selected:continue
    shutil.copy2(STAGE/f'{slug}-cartoon-hands.png',prepared/f'{slug}-cartoon-hands.png')
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    for image in bpy.data.images:
        name=image.filepath.replace('\\','/').rsplit('/',1)[-1]
        if (ASSETS/name).is_file() or (prepared/name).is_file():image.filepath='//'+name
    bpy.context.preferences.filepaths.save_version=0
    # Some Windows readers deny Blender's rename-over-existing save. Prepare
    # a complete file first, then copy it over the already snapshotted asset.
    bpy.ops.wm.save_as_mainfile(filepath=str(prepared/f'{slug}.blend'),relative_remap=False)
    export_fbx(bpy.data.objects[f'{slug}_SkinnedMesh'],bpy.data.objects['TSB_Rig'],prepared/f'{slug}.fbx')
    for suffix in ('-hands.json','-rig.json'):shutil.copy2(STAGE/f'{slug}{suffix}',prepared/f'{slug}{suffix}')
    print('CARTOON_HANDS_READY',slug,flush=True)
