"""Publish verified facial changes while preserving all current rig/mesh data."""
import bpy,hashlib,json,shutil,sys
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
OUT=ROOT/'artifacts/blink-surface-fix';ASSETS=ROOT/'assets/characters'
verified=json.loads((OUT/'asset-verification.json').read_text())
slugs=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else list(verified)

def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

# Check the whole publication set before writing anything. Do not overwrite
# character edits made while the candidate was being generated and reviewed.
for slug in slugs:
    assert verified[slug]['unchanged_neutral_uv_weights_bones_other_shapes']
    for suffix in ['.blend','.fbx','.morphs.json','-rig.json','-blink-L.png','-blink-R.png','-blink-half.png','-blink-closed.png']:
        assert digest(ASSETS/f'{slug}{suffix}')==digest(OUT/'before'/f'{slug}{suffix}'),(slug,suffix,'source changed')

for slug in slugs:
    bpy.ops.wm.open_mainfile(filepath=str(OUT/'candidate'/f'{slug}.blend'))
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
    for suffix in ['-blink-L.png','-blink-R.png','-blink-half.png','-blink-closed.png']:
        shutil.copy2(OUT/'candidate'/f'{slug}{suffix}',ASSETS/f'{slug}{suffix}')
    for image in bpy.data.images:
        name=Path(bpy.path.abspath(image.filepath)).name
        if image.source=='FILE' and (ASSETS/name).is_file():image.filepath='//'+name
    bpy.context.scene.frame_set(0)
    for key in body.data.shape_keys.key_blocks:key.value=0
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/f'{slug}.blend'),relative_remap=False)
    export_fbx(body,rig,ASSETS/f'{slug}.fbx')
    record=json.loads((ASSETS/f'{slug}-rig.json').read_text())
    basis=np.array([v.co[:] for v in body.data.vertices])
    for key in body.data.shape_keys.key_blocks:
        if key.name.startswith('eyeBlink_'):
            target=np.array([v.co[:] for v in key.data])
            record['face']['morph_vertices'][key.name]=int(np.count_nonzero(np.linalg.norm(target-basis,axis=1)>1e-6))
    record['blink_surface']=dict(revision='smooth-depth-and-normal-v1',
                                normal_targets=True,outer_corner_erasure=True,
                                protected_accessories=slug in ['ui','hilde'])
    (ASSETS/f'{slug}-rig.json').write_text(json.dumps(record,indent=2))
    print('BLINK_PUBLISHED',slug,flush=True)
