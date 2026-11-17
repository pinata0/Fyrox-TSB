"""Publish the reviewed Tripo candidate to the existing Butter game paths (Blender)."""
import bpy
import json
import hashlib
import shutil
import sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'artifacts/tripo-butter'
OUT=ROOT/'assets/characters'
verification=json.loads((SOURCE/'verification.json').read_text())
assert verification['passed']
assert all(hashlib.sha256((SOURCE/name).read_bytes()).hexdigest()==digest for name,digest in verification['candidate_sha256'].items())
assert (ROOT/'assets/character-sources/tripo/butter/source-files.json').is_file()
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
bpy.ops.wm.open_mainfile(filepath=str(SOURCE/'butter.blend'))
for kind in ['rigged','clothes','accessories']:
    filename=f'butter-{kind}-basecolor.jpg'
    # Fyrox RecursiveUp searches the model's own directory and its parents, not child directories.
    shutil.copy2(SOURCE/'textures'/filename,OUT/filename)
    bpy.data.images[f'butter-{kind}-basecolor'].filepath=str(OUT/filename)
    obsolete=OUT/'textures'/filename
    if obsolete.exists() and obsolete.read_bytes()==(OUT/filename).read_bytes():
        obsolete.unlink()
if (OUT/'textures').is_dir() and not any((OUT/'textures').iterdir()):
    (OUT/'textures').rmdir()
bpy.context.scene.frame_set(0)
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'butter.blend'))
export_fbx(bpy.data.objects['butter_SkinnedMesh'],bpy.data.objects['TSB_Rig'],OUT/'butter.fbx')
for filename in ['butter-preview.png','butter-portrait.png']:
    shutil.copy2(SOURCE/filename,OUT/filename)
records=json.loads((OUT/'manifest.json').read_text(encoding='utf-8'))
record=next(r for r in records if r['slug']=='butter')
record.update(json.loads((SOURCE/'candidate.json').read_text()))
(OUT/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2),encoding='utf-8')
print('TRIPO_BUTTER_PUBLISHED',flush=True)
