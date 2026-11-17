"""Publish validated staged Meshy assets, preserving the previous runtime files."""
import hashlib,json,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/meshy-import';STAGE=OUT/'characters';DEST=ROOT/'assets/characters'
def digest(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def main():
    verification=json.loads((OUT/'verification.json').read_text())
    assert verification['passed'] and len(verification['characters'])==15,'All staged rigs must pass first'
    manifest=json.loads((DEST/'manifest.json').read_text())
    backup=OUT/'before/assets/characters';backup.mkdir(parents=True,exist_ok=True)
    delivered=[]
    for p in sorted(STAGE.iterdir()):
        if not p.is_file() or p.suffix not in ['.blend','.fbx','.png','.json']:continue
        target=DEST/p.name
        if target.exists() and not (backup/p.name).exists():shutil.copy2(target,backup/p.name)
        shutil.copy2(p,target);delivered.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=digest(target),bytes=target.stat().st_size))
    faces=[]
    for entry in manifest:
        slug=entry['slug'];record=json.loads((STAGE/f'{slug}-rig.json').read_text())
        for key in ['vertices','triangles','bones','clips','fps']:entry[key]=record[key]
        entry['source']='Meshy generated files supplied by user, 2026-11-15'
        entry['morphs']=list(record['face']['morph_vertices'])
        entry['companions']=record.get('companions',[])
        entry['sha256']['blend']=digest(DEST/f'{slug}.blend');entry['sha256']['model']=digest(DEST/f'{slug}.fbx')
        if (DEST/f'{slug}-weapon.fbx').exists():entry['weapon']=f'assets/characters/{slug}-weapon.fbx'
        f=record['face'];faces.append([f['eye_x']+.055,f['mouth_y']+.02,f['front'],f['eye_y'],f['eye_x'],f['mouth_y']])
    (DEST/'manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False),encoding='utf-8')
    (DEST/'meshy-faces.json').write_text(json.dumps(faces,indent=2))
    (OUT/'delivery.json').write_text(json.dumps(dict(date='2026-11-15',files=delivered,originals_preserved=True),indent=2))
    print('MESHY_PUBLISHED',len(delivered),'files')

if __name__=='__main__':main()
