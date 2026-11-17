"""Download research references only; these images are never shipped as game textures."""
import hashlib
import json
from pathlib import Path
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/references/fairy-kingdom-castle'
REFERENCES = [
    dict(file='official-world.png', title='Trickcal official global site: Elias castle and world tree',
         page='https://trickcal.biligames.com/en/',
         image='https://creator-static.biligames.com/07bb6d1a0cbd91e9a8b303c9fabd78daa1b5dba7ea832d408de8d6d66abd3a18.png',
         use='Castle silhouette, mint roofs, ivory stone, gold trim, giant world tree'),
    dict(file='royal-hall.png', title='In-game royal hall screenshot reproduced in an Elfin story article',
         page='https://note.com/little_mm_1009/n/n8f1e42b6a9ed',
         image='https://assets.st-note.com/img/1768315528-smuI9iG6S4XMvrHkAn3YCjzf.png',
         use='Royal hall columns, wall ornament and warm palette; secondary screenshot source'),
    dict(file='anubis-layout.jpg', title='Temple of Anubis annotated overhead layout',
         page='https://eu.forums.blizzard.com/de/overwatch/t/guide-tool-overwatch-maps-karten-draufsichten-%F0%9F%8C%86/8975',
         image='https://3.bp.blogspot.com/-c2XlpEPPMZA/Vz_nygDOrpI/AAAAAAAAAeo/jbsGlLZvdrsBxqMwbFGmB84lN4zqgrG7gCLcB/s1600/templeofanubis-maplayout.jpg',
         use='A gateway/high ground, B three approaches, forward attack and rear defense spawn'),
    dict(file='anubis-point-a.gif', title='Temple of Anubis A three-dimensional diagram by Orhin_InsaLan',
         page='https://www.furiouspaul.com/overwatch/temple-of-anubis/',
         image='https://www.furiouspaul.com/overwatch/temple-of-anubis/capture-a-temple-of-anubis.gif',
         use='A balcony and stair/flank relationships'),
    dict(file='anubis-point-b.gif', title='Temple of Anubis B three-dimensional diagram by Orhin_InsaLan',
         page='https://www.furiouspaul.com/overwatch/temple-of-anubis/',
         image='https://www.furiouspaul.com/overwatch/temple-of-anubis/capture-b-temple-of-anubis.gif',
         use='B courtyard, pillars and surrounding routes'),
]

if __name__ == '__main__':
    OUT.mkdir(parents=True, exist_ok=True)
    for ref in REFERENCES:
        path = OUT / ref['file']
        try:
            if not path.exists():
                request = Request(ref['image'], headers={'User-Agent': 'Mozilla/5.0'})
                path.write_bytes(urlopen(request, timeout=45).read())
            ref['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
            ref['bytes'] = path.stat().st_size
            print('REFERENCE_OK', ref['file'], ref['bytes'])
        except Exception as exc:
            ref['error'] = str(exc)
            print('REFERENCE_FAILED', ref['file'], exc)
    (OUT / 'sources.json').write_text(json.dumps({'accessed': '2026-11-13', 'usage': 'Research only; original game assets modeled separately', 'references': REFERENCES}, ensure_ascii=False, indent=2), encoding='utf-8')
