"""Archive the user's delivery by identity and create independent staging files."""
import hashlib
import json
import shutil
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
DEST=ROOT/'assets/character-sources/lowpoly'
WORK=ROOT/'artifacts/lowpoly-import'
KINDS={
 'butter_slingshot':('butter','weapon'), 'Chibi_Candy_Girl':('picora','character'),
 'chibi_cat_girl':('komi','character'), 'Chibi_Character_Remesh':('bella','character'),
 'Chibi_Dragon_Girl':('vivi','character'), 'chibi_dragon_remesh':('shion','character'),
 'chibi_elf_remesh':('id','character'), 'Chibi_Fairy':('mayo','character'),
 'Chibi_Frog_Raincoat':('ui','character'), 'chibi_royal':('erpin','character'),
 'chibi_sunhat':('naia','character'), 'chloe_chibi':('chloe','character'),
 'clover_wand':('ui','weapon'), 'dolphin_water':('naia','weapon'),
 'Flower_Petal_Parasol':('aya','weapon'), 'golden_clock':('renua','weapon'),
 'Medical_Injector':('hilde','weapon'), 'Mercury_Blade':('vivi','weapon'),
 'nata_companion':('id','robot'), 'pink_ruffled':('komi','weapon'),
 'Remeshed_Chibi_Elf':('hilde','character'), 'Chibi_Kimono':('aya','character'),
 'Chibi_Puppy':('butter','character'), 'Crescent_Moon':('id','mount'),
 'Curled_Staff':('picora','weapon'), 'Frog_Mount':('ui','companion'),
 'Ghost_Flame':('bella','weapon'), 'RemeshedChibiCharacter':('renua','character'),
 'royal_staff':('erpin','weapon'), 'Sebastian_Bear':('chloe','companion'),
 'stylized_blowpipe':('mayo','weapon'), 'stylized_sniper':('shion','weapon'),
}
def digest(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

if __name__=='__main__':
    manifest=DEST/'manifest.json'
    if not manifest.exists():
        entries=[]
        files=sorted((ROOT/'temp').glob('*.blend'));assert len(files)==32,len(files)
        for source in files:
            found=[value for key,value in KINDS.items() if key.lower() in source.name.lower()]
            assert len(found)==1,(source,found)
            slug,kind=found[0];target=DEST/slug/f'{slug}-{kind}-source.blend'
            assert source.resolve().is_relative_to((ROOT/'temp').resolve())
            assert target.resolve().is_relative_to(DEST.resolve()) and not target.exists()
            entries.append(dict(slug=slug,kind=kind,original=source.relative_to(ROOT).as_posix(),
                source=target.relative_to(ROOT).as_posix(),sha256=digest(source)))
        for row in entries:
            source=ROOT/row['original'];target=ROOT/row['source'];target.parent.mkdir(parents=True,exist_ok=True)
            shutil.move(str(source),str(target));assert digest(target)==row['sha256']
        manifest.write_text(json.dumps(dict(date='2026-11-21',entries=entries),indent=2)+'\n')
    for folder in ['before','characters']:(WORK/folder).mkdir(parents=True,exist_ok=True)
    for source in (ROOT/'assets/characters').iterdir():
        if not source.is_file():continue
        backup=WORK/'before'/source.name
        if not backup.exists():shutil.copy2(source,backup)
        staged=WORK/'characters'/source.name
        if not staged.exists():shutil.copy2(source,staged)
    print('LOWPOLY_SOURCES_ORGANIZED',len(json.loads(manifest.read_text())['entries']),flush=True)
