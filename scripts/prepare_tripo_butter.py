"""Back up and give the uploaded FBXs and their texture links stable names. Run in Blender."""
import json
import shutil
import hashlib
from datetime import datetime
from pathlib import Path
from io_scene_fbx import parse_fbx, encode_bin

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'assets/character-sources/tripo/butter'
receipt = SOURCE / 'source-files.json'
if receipt.exists():
    print('TRIPO_SOURCE_ALREADY_PREPARED')
else:
    prior = sorted((ROOT / 'artifacts/tripo-backups').glob('*/source-originals'))
    backup = prior[0].parent if prior else ROOT / 'artifacts/tripo-backups' / datetime.now().strftime('%Y%m%d-%H%M%S')
    if not prior:
        shutil.copytree(SOURCE, backup / 'source-originals')
        (backup / 'previous-character').mkdir()
        for name in ['butter.blend','butter.fbx','butter-preview.png','butter-portrait.png','manifest.json']:
            shutil.copy2(ROOT / 'assets/characters' / name, backup / 'previous-character' / name)
    mapping = []
    methods = {'B':'add_bool','C':'add_char','Z':'add_int8','Y':'add_int16','I':'add_int32','L':'add_int64',
               'F':'add_float32','D':'add_float64','R':'add_bytes','S':'add_string',
               'i':'add_int32_array','l':'add_int64_array','f':'add_float32_array',
               'd':'add_float64_array','b':'add_bool_array','c':'add_byte_array'}
    for kind in ['rigged','clothes','accessories']:
        directory = SOURCE / kind
        old_fbx = next((backup / 'source-originals' / kind).glob('*.fbx'))
        old_jpg = next(directory.rglob('*.jpg'))
        name = f'butter-{kind}'
        new_fbx = directory / f'{name}.fbx'
        new_texture_dir = directory / f'{name}.fbm'
        new_texture_dir.mkdir(exist_ok=True)
        new_jpg = new_texture_dir / f'{name}-basecolor.jpg'
        old_texture_dir = old_jpg.parent
        if old_jpg != new_jpg:
            old_jpg.rename(new_jpg)
            old_texture_dir.rmdir()
        relative = f'{new_texture_dir.name}/{new_jpg.name}'.encode('utf-8')
        tree, version = parse_fbx.parse(str(old_fbx))
        changes = []
        def copy_element(elem):
            out = encode_bin.FBXElem(elem.id)
            for t, value in zip(elem.props_type,elem.props):
                if t == ord('B'):
                    value = bool(value)
                if t == ord('S') and elem.id in [b'FileName', b'Filename', b'RelativeFilename'] and value.lower().endswith((b'.jpg',b'.png')):
                    changes.append(value.decode('utf-8',errors='replace'))
                    value = relative
                getattr(out,methods[chr(t)])(value)
            out.elems = [copy_element(child) for child in elem.elems]
            return out
        encode_bin.write(str(new_fbx),copy_element(tree),version)
        assert changes, (kind,'missing texture links')
        original_working = directory / old_fbx.name
        if original_working.exists() and original_working != new_fbx:
            original_working.unlink()
        mapping.append({'kind':kind,'original_fbx':old_fbx.name,'fbx':new_fbx.relative_to(ROOT).as_posix(),
                        'texture':new_jpg.relative_to(ROOT).as_posix(), 'texture_links_updated':len(changes),
                        'sha256':hashlib.sha256(new_fbx.read_bytes()).hexdigest()})
    archive=next(SOURCE.glob('*.zip'), None)
    if archive and archive.name != 'butter-rigged-original.zip':
        archive.rename(SOURCE/'butter-rigged-original.zip')
    receipt.write_text(json.dumps({'backup':backup.relative_to(ROOT).as_posix(),'files':mapping},ensure_ascii=False,indent=2),encoding='utf-8')
    print('TRIPO_SOURCE_PREPARED',backup,flush=True)
