"""Preserve originals and unpack/rename Meshy production sources safely."""
import hashlib
import json
import shutil
import zipfile
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'assets/character-sources/meshy'
IDENTITIES=['picora','komi','bella','vivi','shion','id','hilde','mayo','ui','aya','erpin','naia','chloe','ui-frog','butter','renua','chloe-sebastian']
PROPS={'clover_wand':('ui','weapon'),'crescent_moon':('id','mount'),
       'curled_staff':('picora','weapon'),'dolphin_water':('naia','weapon'),
       'Ghost_Flame':('bella','weapon'),'medical_injector':('hilde','weapon'),
       'Mercury_Blade':('vivi','weapon'),'Parasol':('aya','weapon'),
       'Pink_Ruffled_Pillow':('komi','weapon'),'butter_slingshot':('butter','weapon'),
       'Magic_Circle':('renua','weapon'),'Remeshed_Robot':('id','robot'),
       'RoyalStaff':('erpin','weapon'),'stylized_blowpipe':('mayo','weapon'),
       'stylized_sniper':('shion','weapon')}

def digest(p):
    return hashlib.file_digest(p.open('rb'),'sha256').hexdigest()

def main():
    inventory=json.loads((ROOT/'artifacts/meshy-import/inspection/inventory.json').read_text())
    assert len(inventory)==len(IDENTITIES)
    records=[]
    for item,slug in zip(inventory,IDENTITIES):
        source=ROOT/'assets/temp'/item['file']
        folder=SOURCE/slug;folder.mkdir(parents=True,exist_ok=True)
        target=folder/f'{slug}-source.glb'
        if not target.exists():shutil.copy2(source,target)
        assert digest(source)==digest(target)
        records.append(dict(slug=slug,kind='character' if '-' not in slug else 'companion',original=source.relative_to(ROOT).as_posix(),source=target.relative_to(ROOT).as_posix(),sha256=digest(target)))
    for source in sorted((ROOT/'assets/temp').glob('*.zip')):
        matched=[v for key,v in PROPS.items() if key.lower() in source.name.lower()]
        assert len(matched)==1,source
        slug,kind=matched[0];folder=SOURCE/slug/kind;folder.mkdir(parents=True,exist_ok=True)
        files=[]
        with zipfile.ZipFile(source) as archive:
            for entry in archive.infolist():
                if entry.is_dir():continue
                basename=Path(entry.filename).name
                suffix=next((s for s in ['_metallic_roughness.png','_metallic.png','_roughness.png','_normal.png'] if basename.endswith(s)),None)
                suffix=suffix or ('-basecolor.png' if basename.endswith('.png') else Path(basename).suffix)
                target=folder/f'{slug}-{kind}{suffix}'
                assert target.resolve().is_relative_to(SOURCE.resolve())
                with archive.open(entry) as src,target.open('wb') as dst:shutil.copyfileobj(src,dst)
                files.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=digest(target)))
        records.append(dict(slug=slug,kind=kind,original=source.relative_to(ROOT).as_posix(),source=(folder/f'{slug}-{kind}.fbx').relative_to(ROOT).as_posix(),sha256=digest(source),files=files))
    (SOURCE/'manifest.json').write_text(json.dumps(dict(date='2026-11-15',entries=records),indent=2),encoding='utf-8')
    print('MESHY_SOURCES_OK',len(records))

if __name__=='__main__':main()
