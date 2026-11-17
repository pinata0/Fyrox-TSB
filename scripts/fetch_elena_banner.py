"""Download original Elena art and losslessly transcode it for the map's FBX.

Requires Pillow. No resizing, cropping, recolouring or generated artwork.
"""
import hashlib
import json
import sys
from pathlib import Path
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
# Reuse the project's existing isolated Pillow install when available.
sys.path.insert(0, str(ROOT / 'artifacts/trait-markers/python-deps'))
from PIL import Image

REFERENCES = ROOT / 'assets/references/elf-city'
OUT = ROOT / 'assets/maps/elf-city'
FILES = [
    ('elena-fullbody.webp', 'https://www.gacha.gg/img/trickcal/dm/chars/elena-standing.webp'),
    ('cityhall-original.webp', 'https://www.gacha.gg/img/trickcal/dm/scenes/elf_cityhall_night.webp'),
]


def main():
    records = []
    for filename, url in FILES:
        data = urlopen(Request(url, headers={'User-Agent': 'Mozilla/5.0'}), timeout=30).read()
        path = REFERENCES / filename
        path.write_bytes(data)
        with Image.open(path) as img:
            img.load()
            records.append(dict(path=path.relative_to(ROOT).as_posix(), url=url,
                                size=list(img.size), sha256=hashlib.sha256(data).hexdigest()))
    source = REFERENCES / FILES[0][0]
    texture = OUT / 'elena-banner.png'
    with Image.open(source) as img:
        rgba = img.convert('RGBA')
        assert rgba.height >= 600 and rgba.width / rgba.height < .7
        assert rgba.getchannel('A').getextrema() == (0, 255)
        rgba.save(texture)
        with Image.open(texture) as png:
            assert png.tobytes() == rgba.tobytes(), 'PNG must preserve the original pixels'
    manifest = dict(
        accessed='2026-11-15', publisher='EPID Games', host='gacha.gg (unofficial game-art archive)',
        character_page='https://www.gacha.gg/trickcal/characters/elena',
        background_page='https://www.gacha.gg/trickcal/story/elena-s-story',
        lore_page='https://d.namu.moe/w/오르(트릭컬 리바이브)',
        evidence='Original city-hall background shows a large vertical Elena portrait above the entrance. '
                 'Orr story synopsis identifies Orr as the portrait artist.',
        adaptation='The map uses the intact full-body standing illustration, not a low-resolution crop '
                   'of the painting embedded in the background. Placement follows that background; '
                   'the standing pose is not the identical painted portrait.',
        files=records,
        runtime=dict(path=texture.relative_to(ROOT).as_posix(), size=list(rgba.size),
                     sha256=hashlib.sha256(texture.read_bytes()).hexdigest(),
                     conversion='Lossless RGBA PNG, original dimensions and alpha; no crop or redraw'),
        rights='Original Trickcal artwork belongs to EPID Games. No ownership or license transfer is implied.',
    )
    (OUT / 'elena-banner-source.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print('ELENA_TEXTURE_OK', rgba.size, texture)


if __name__ == '__main__':
    main()
