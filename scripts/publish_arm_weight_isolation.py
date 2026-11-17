"""Apply the independently validated arm-domain correction to runtime assets."""
import bpy, hashlib, json, shutil, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from export_fyrox_fbx import export_fbx
OUT = ROOT / 'artifacts/arm-isolation'
STAGE = OUT / 'characters'
ASSETS = ROOT / 'assets/characters'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


report = json.loads((OUT / 'verification.json').read_text())
assert report['passed'] and len(report['characters']) == 15
for entry in report['characters']:
    slug = entry['slug']
    rig_record = json.loads((STAGE / f'{slug}-rig.json').read_text())
    for suffix, expected in rig_record['arm_weight_isolation']['source_sha256'].items():
        assert digest(ASSETS / f'{slug}{suffix}') == expected, (slug, suffix, 'source changed during review')
    for suffix, expected in entry['sha256'].items():
        assert digest(STAGE / f'{slug}{suffix}') == expected, (slug, suffix, 'stage changed after validation')
shutil.copy2(OUT / 'verification.json', OUT / 'stage-verification.json')
published = {}
for entry in report['characters']:
    slug = entry['slug']
    bpy.ops.wm.open_mainfile(filepath=str(STAGE / f'{slug}.blend'))
    for image in bpy.data.images:
        filename = image.filepath.replace('\\', '/').rsplit('/', 1)[-1]
        if filename and (ASSETS / filename).is_file():
            image.filepath = '//' + filename
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS / f'{slug}.blend'), relative_remap=False)
    body, rig = bpy.data.objects[f'{slug}_SkinnedMesh'], bpy.data.objects['TSB_Rig']
    export_fbx(body, rig, ASSETS / f'{slug}.fbx')
    shutil.copy2(STAGE / f'{slug}-rig.json', ASSETS / f'{slug}-rig.json')
    published[slug] = json.loads((ASSETS / f'{slug}-rig.json').read_text())['arm_weight_isolation']
    print('ARM_ISOLATION_PUBLISHED', slug, flush=True)
# Re-read after model work so independent manifest changes remain intact.
path = ASSETS / 'manifest.json'
records = json.loads(path.read_text(encoding='utf-8'))
for record in records:
    if record['slug'] in published:
        record['arm_weight_isolation'] = published[record['slug']]
        for field in ('blend', 'model'):
            record['sha256'][field] = digest(ROOT / record[field])
path.write_text(json.dumps(records, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
