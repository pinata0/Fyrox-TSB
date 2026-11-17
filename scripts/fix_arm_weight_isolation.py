"""Stage the roster repair from current assets, with immutable input backups.

Run in Blender background, optionally with -- followed by character slugs.
Publish only after verify_arm_weight_isolation.py and visual review.
"""
import bpy, hashlib, json, shutil, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from arm_weight_isolation import isolate_refined_arm_weights, REVISION
from export_fyrox_fbx import export_fbx, resolve_character_textures

ASSETS = ROOT / 'assets/characters'
OUT = ROOT / 'artifacts/arm-isolation'
STAGE = OUT / 'characters'
BACK = OUT / 'before'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    STAGE.mkdir(parents=True, exist_ok=True)
    BACK.mkdir(parents=True, exist_ok=True)
    selected = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
    records = json.loads((ASSETS / 'manifest.json').read_text(encoding='utf-8'))
    if not (BACK / 'manifest.json').exists():
        shutil.copy2(ASSETS / 'manifest.json', BACK / 'manifest.json')
    for record in records:
        slug = record['slug']
        if selected and slug not in selected:
            continue
        for suffix in ('.blend', '.fbx', '-rig.json', '.morphs.json'):
            path = ASSETS / f'{slug}{suffix}'
            backup = BACK / path.name
            if not backup.exists():
                shutil.copy2(path, backup)
        bpy.ops.wm.open_mainfile(filepath=str(BACK / f'{slug}.blend'))
        body = bpy.data.objects[f'{slug}_SkinnedMesh']
        rig = bpy.data.objects['TSB_Rig']
        report = isolate_refined_arm_weights(body, rig)
        report.update(revision=REVISION, slug=slug, source_sha256={
            suffix: digest(BACK / f'{slug}{suffix}') for suffix in ('.blend', '.fbx', '-rig.json', '.morphs.json')})
        meshes = [o for o in rig.children if o.type == 'MESH']
        # A backup is in a different directory; resolve // links against the
        # source asset folder before the exporter's packed-image fallback.
        for image in bpy.data.images:
            filename = image.filepath.replace('\\', '/').rsplit('/', 1)[-1]
            if filename and (ASSETS / filename).is_file():
                image.filepath = str(ASSETS / filename)
        resolve_character_textures(meshes)
        for image in bpy.data.images:
            if image.source == 'FILE' and image.filepath:
                image.filepath = bpy.path.abspath(image.filepath)
        bpy.context.scene.frame_set(0)
        bpy.context.preferences.filepaths.save_version = 0
        bpy.ops.wm.save_as_mainfile(filepath=str(STAGE / f'{slug}.blend'))
        export_fbx(body, rig, STAGE / f'{slug}.fbx')
        record = json.loads((BACK / f'{slug}-rig.json').read_text())
        record['arm_weight_isolation'] = report
        (STAGE / f'{slug}-rig.json').write_text(json.dumps(record, ensure_ascii=False, indent=2), encoding='utf-8')
        print('ARM_ISOLATION_STAGED', slug, json.dumps(report), flush=True)


if __name__ == '__main__':
    main()
