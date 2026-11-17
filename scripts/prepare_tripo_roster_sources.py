"""Give uploaded FBXs, textures and archives stable names; run in Blender."""
import hashlib
import json
import shutil
from pathlib import Path
from zipfile import ZipFile
from io_scene_fbx import parse_fbx, encode_bin

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / 'artifacts/tripo-roster-import'
SPECS = json.loads((ROOT/'scripts/tripo_roster_sources.json').read_text())
METHODS = dict(zip('BCZYILFDRSilfdbc', [
    'add_bool','add_char','add_int8','add_int16','add_int32','add_int64',
    'add_float32','add_float64','add_bytes','add_string','add_int32_array',
    'add_int64_array','add_float32_array','add_float64_array','add_bool_array','add_byte_array']))

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

for slug, spec in SPECS.items():
    source = ROOT/'assets/character-sources/tripo'/slug
    source.mkdir(parents=True,exist_ok=True)
    records=[]
    for kind, original in spec['parts'].items():
        extracted=WORK/'extracted'/original
        archive=ROOT/'assets/temp'/f'{original}.zip'
        stored=source/'archives'/f'{slug}-{kind}-original.zip'
        if not extracted.exists():
            extracted.mkdir(parents=True)
            with ZipFile(archive if archive.exists() else stored) as bundle:
                for item in bundle.infolist():
                    assert (extracted/item.filename).resolve().is_relative_to(extracted.resolve())
                    bundle.extract(item,extracted)
        old_fbx=next(extracted.glob('*.fbx'))
        old_texture=next(extracted.rglob('*.jpg'))
        stem=f'{slug}-{kind}'
        folder=source/kind
        texture=folder/f'{stem}.fbm'/f'{stem}-basecolor.jpg'
        texture.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(old_texture,texture)
        relative=f'{stem}.fbm/{texture.name}'.encode()
        tree,version=parse_fbx.parse(str(old_fbx))
        changes=[]
        def copy_element(elem):
            result=encode_bin.FBXElem(elem.id)
            for typ,value in zip(elem.props_type,elem.props):
                if typ==ord('B'): value=bool(value)
                if typ==ord('S') and elem.id in [b'FileName',b'Filename',b'RelativeFilename'] and value.lower().endswith((b'.jpg',b'.png')):
                    changes.append(value.decode(errors='replace'))
                    value=relative
                getattr(result,METHODS[chr(typ)])(value)
            result.elems=[copy_element(e) for e in elem.elems]
            return result
        output=folder/f'{stem}.fbx'
        encode_bin.write(str(output),copy_element(tree),version)
        assert changes
        stored.parent.mkdir(exist_ok=True)
        if archive.exists():
            if stored.exists(): assert digest(archive)==digest(stored)
            else: shutil.copy2(archive,stored)
            assert digest(archive)==digest(stored)
            archive.unlink()
        assert stored.exists()
        records.append({'kind':kind,'original_archive':f'{original}.zip',
                        'archive':stored.relative_to(ROOT).as_posix(),'archive_sha256':digest(stored),
                        'fbx':output.relative_to(ROOT).as_posix(),'sha256':digest(output),
                        'texture':texture.relative_to(ROOT).as_posix(),
                        'texture_links_updated':len(changes),'active':True,
                        'attachment':'weapon' if kind in ['weapon','weapon-hat'] else 'character'})
    (source/'source-files.json').write_text(json.dumps({'files':records},indent=2),encoding='utf-8')
print('ROSTER_SOURCES_PREPARED: 19 archives, named FBXs and repaired texture links',flush=True)
