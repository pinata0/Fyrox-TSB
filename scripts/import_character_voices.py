"""Import Korean base-character voices and their published dialogue metadata.

Run again with --offline to verify cached sources and regenerate the embedded bank.
Combat keys/text come from The Master's Journey's public Korean locale files;
their audio uses the same verified Cheeky archive layout as the wiki recordings.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import re
import subprocess
import time
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/audio/characters'
CACHE = ROOT / 'artifacts/character-voice-sources'
WIKI = 'https://wiki.cheeky.tsuncloud.com/korea/apostles/'
LOCALES = 'https://journey.927927927.xyz/locales/ko-KR/characters/'
# Stable game roster order; the archive uses different spellings for five heroes.
ROSTER = [('vivi', 'vivi'), ('naia', 'naia'), ('erpin', 'erpin'), ('id', 'ed'),
          ('aya', 'aya'), ('picora', 'picora'), ('chloe', 'chloe'),
          ('renua', 'renewaawaken'), ('mayo', 'mayo'), ('bella', 'vela'),
          ('ui', 'ui'), ('butter', 'butter'), ('komi', 'kommy'),
          ('hilde', 'hilde'), ('shion', 'xxionx')]
NAMES = ['비비', '나이아', '에르핀', '이드', '아야', '피코라', '클로에', '리뉴아',
         '마요', '벨라', '우이', '버터', '코미', '힐데', '시온 더 다크불릿']
TRIGGERS = {'Select': '캐릭터 선택', 'Spawn': '전투 입장 / 부활', 'Start': '라운드 시작',
            'Win': '승리', 'Lose': '패배', 'Death': '사망', 'Cheek': '선택 화면에서 볼 당기기',
            'Line': 'C 소통 휠 → 음성 대사', 'Skill': 'E 스킬 시전', 'Ultimate': 'Q 궁극기 시전'}
CUES = {'Select': ('DeckSetting1', 'DeckSetting2'), 'Spawn': ('Spawn1', 'Spawn2'),
        'Start': ('Stage',), 'Win': ('Victory1', 'Victory2'),
        'Lose': ('Defeat1', 'Defeat2'), 'Death': ('Die1', 'Die2'),
        'Cheek': ('Touch1', 'Touch1_1', 'Touch1_2'),
        'Line': ('Affinity1_1', 'Affinity1_2', 'Affinity1_3'),
        'Skill': ('SPSkill1', 'SPSkill3'), 'Ultimate': ('Ultimate3', 'Ultimate1')}
# Some performances are stored as separate takes/phrases. Only verified complete
# utterances are selected; missing aggregate names are not substituted with upgrades.
CAST_OVERRIDES = {
    'vivi': {'Ultimate': ('Ultimate3',)},
    'naia': {'Ultimate': ('Ultimate3',)},
    'aya': {'Skill': ('SPSkill1_1_1', 'SPSkill3_1'),
            'Ultimate': ('Ultimate1_2', 'Ultimate3_1')},
    'chloe': {'Skill': ('SPSkill1_1', 'SPSkill3_1'), 'Ultimate': ('Ultimate3',)},
    'mayo': {'Ultimate': ('Ultimate1',)},
    'ui': {'Skill': ('SPSkill1_1', 'SPSkill3'), 'Ultimate': ('Ultimate1',)},
    'hilde': {'Ultimate': ('Ultimate3',)},
}


def fetch(url):
    parsed = urllib.parse.urlparse(url)
    if parsed.scheme != 'https' or parsed.hostname not in {
            'wiki.cheeky.tsuncloud.com', 'cheeky.tsuncloud.com', 'journey.927927927.xyz'}:
        raise ValueError(f'Unexpected source URL: {url}')
    for attempt in range(3):
        try:
            request = urllib.request.Request(url, headers={'User-Agent': 'TSB-voice-import/1.0'})
            with urllib.request.urlopen(request, timeout=40) as response:
                return response.read()
        except OSError:
            if attempt == 2:
                raise
            time.sleep(attempt + 1)


def voice_lines(markup):
    chunks = []
    for encoded in re.findall(r'self\.__next_f\.push\((\[.*?\])\)</script>', markup):
        chunk = json.loads(encoded)
        if len(chunk) == 2 and chunk[0] == 1 and isinstance(chunk[1], str):
            chunks.append(chunk[1])
    stream = ''.join(chunks)
    marker = '"lines":['
    if marker not in stream:
        raise ValueError('Voice list missing from page')
    return json.JSONDecoder().raw_decode(stream[stream.index(marker) + len('"lines":'):])[0]


def import_hero(item, offline):
    hero, (slug, source_slug) = item
    page = WIKI + source_slug
    cached = CACHE / f'{slug}.html'
    if not cached.exists():
        if offline:
            raise FileNotFoundError(cached)
        cached.write_bytes(fetch(page))
    lines = voice_lines(cached.read_text(encoding='utf-8-sig'))
    reference = next(line for line in lines if line['key'].endswith('_Spawn1'))
    code = reference['key'].split('_')[0]
    text_source = LOCALES + code + 'Voice.json'
    locale = CACHE / 'cast-research' / f'{slug}-voice-locale.json'
    if not locale.exists():
        if offline:
            raise FileNotFoundError(locale)
        locale.parent.mkdir(parents=True, exist_ok=True)
        locale.write_bytes(fetch(text_source))
    texts = json.loads(locale.read_text(encoding='utf-8'))
    cues = CUES | CAST_OVERRIDES.get(slug, {})
    for cue in ('Skill', 'Ultimate'):
        for suffix in cues[cue]:
            key = code + '_' + suffix
            original, replacement = 'Voice_' + reference['key'], 'Voice_' + key
            url = reference['audio']['kor'].replace(original.lower(), replacement.lower()).replace(
                original, replacement)
            lines.append(dict(key=key, title='저학년 시전' if cue == 'Skill' else '고학년 시전',
                              text=texts[key], text_source=text_source, audio={'kor': url}))
    records = []
    for cue, suffixes in cues.items():
        for suffix in suffixes:
            matches = [line for line in lines if line['key'].lower().endswith('_' + suffix.lower())
                       and '_skin' not in line['key'].lower() and line.get('audio', {}).get('kor')]
            if not matches:
                continue
            if len(matches) != 1:
                raise ValueError(f'Ambiguous base voice: {slug}/{suffix}')
            line = matches[0]
            if not re.fullmatch(r'[A-Za-z0-9_]+', line['key']):
                raise ValueError(f'Unexpected voice key: {line["key"]!r}')
            url = line['audio']['kor']
            if '/kor/' not in url:
                raise ValueError(f'Non-Korean audio: {url}')
            target = OUT / slug / (suffix.lower() + '.ogg')
            source = CACHE / 'raw' / slug / target.name
            target.parent.mkdir(parents=True, exist_ok=True)
            source.parent.mkdir(parents=True, exist_ok=True)
            if not source.exists():
                if offline:
                    raise FileNotFoundError(source)
                data = fetch(url)
                if not data.startswith(b'OggS'):
                    raise ValueError(f'Not Ogg audio: {url}')
                source.write_bytes(data)
            # The archive's .ogg files contain Opus, unsupported by Fyrox's decoder.
            # Keep originals in the cache; encode full-length mono Vorbis for runtime.
            if not target.exists() or b'OpusHead' in target.read_bytes()[:128]:
                pending = target.with_suffix('.pending.ogg')
                subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y',
                                '-i', str(source), '-map_metadata', '-1', '-vn',
                                '-ar', '44100', '-ac', '1', '-c:a', 'libvorbis',
                                '-q:a', '5', str(pending)], check=True)
                pending.replace(target)
            data = target.read_bytes()
            if not data.startswith(b'OggS') or len(data) < 1000:
                raise ValueError(f'Invalid audio: {target}')
            records.append(dict(hero=hero, character=slug, speaker=NAMES[hero], cue=cue,
                                trigger=TRIGGERS[cue], key=line['key'],
                                title=line['title'], text=line['text'], page=page, url=url,
                                text_source=line.get('text_source', page),
                                file=target.relative_to(OUT).as_posix(), bytes=len(data),
                                source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
                                sha256=hashlib.sha256(data).hexdigest()))
        if not any(record['cue'] == cue for record in records):
            raise ValueError(f'No Korean base voice for {slug}/{cue}')
    print(f'{slug}: {len(records)} Korean clips', flush=True)
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    CACHE.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=3) as pool:
        records = [r for group in pool.map(lambda item: import_hero(item, args.offline),
                                          enumerate(ROSTER)) for r in group]
    manifest = OUT / 'sources.json'
    if args.offline and manifest.exists():
        previous = json.loads(manifest.read_text(encoding='utf-8'))
        current = {r['file']: r for r in records}
        for old in previous['clips']:
            if old['file'] in current and any(old.get(k) != current[old['file']][k]
                                             for k in ('sha256', 'source_sha256') if k in old):
                raise ValueError(f'Imported file differs from recorded hashes: {old["file"]}')
    manifest.write_text(json.dumps({
        'source': "Trickcal Wiki / Cheeky community audio archive; "
                  "The Master's Journey Korean battle dialogue metadata", 'language': 'Korean',
        'publisher': 'EPID Games', 'retrieved': '2026-11-20',
        'rights': 'Original game voice recordings. Not CC0; rights remain with their owners. '
                  'This source manifest grants no redistribution license.',
        'conversion': 'Source Ogg Opus -> mono Vorbis quality 5, 44100 Hz for Fyrox compatibility; '
                      'full recordings, no trimming or pitch changes. Runtime normalizes peak to 0.8.',
        'clips': records,
    }, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    bank = ['// Generated by scripts/import_character_voices.py; do not edit.',
            'use super::{Clip, Cue};', 'pub(super) const CLIPS: &[Clip] = &[']
    for r in records:
        bank.append(f'    Clip {{ hero: {r["hero"]}, cue: Cue::{r["cue"]}, '
                    f'name: "{r["character"]}/{r["key"]}", '
                    f'speaker: {json.dumps(r["speaker"], ensure_ascii=False)}, '
                    f'text: {json.dumps(r["text"], ensure_ascii=False)}, '
                    f'bytes: include_bytes!("../../../assets/audio/characters/{r["file"]}") }},')
    bank.append('];\n')
    path = ROOT / 'src/presentation/character_voice/bank.rs'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text('\n'.join(bank), encoding='utf-8')
    write_catalog(records)
    print(f'Imported {len(records)} clips, {sum(r["bytes"] for r in records):,} bytes.')


def write_catalog(records):
    def cell(text):
        return text.replace('|', '\\|').replace('\r', '').replace('\n', '<br>')
    rows = ['# 캐릭터 음성 대사 목록', '',
            '누가 **언제 어떤 대사**를 말하는지 정리한 등록 목록입니다. 실제 플레이 이력은 아닙니다.', '',
            f'캐릭터 {len(ROSTER)}명 · 한국어 음성 {len(records)}개. '
            '`scripts/import_character_voices.py`로 음원 등록 정보와 함께 생성합니다.', '',
            '같은 상황에 여러 대사가 있으면 등록 순서대로 번갈아 재생합니다. '
            '음원 링크는 게임에 포함된 파일이며, 원문 표기는 출처의 대사 텍스트를 따릅니다.', '',
            '[재생 규칙](CHARACTER_VOICES.md) · '
            '[기계 판독용 등록 정보](../assets/audio/characters/sources.json)', '']
    for hero, (slug, source_slug) in enumerate(ROSTER):
        text_source = next(r['text_source'] for r in records if r['hero'] == hero and r['cue'] == 'Skill')
        rows.extend([f'## {NAMES[hero]}', '', f'[도감 출처]({WIKI}{source_slug}) · '
                     f'[시전 대사 표기 출처]({text_source})', '',
                     '| 언제 | 무슨 대사 | 원본 분류 | 음원 |',
                     '| --- | --- | --- | --- |'])
        for r in records:
            if r['hero'] == hero:
                rows.append(f'| {r["trigger"]} | {cell(r["text"])} | {cell(r["title"])} | '
                            f'[{r["key"]}](../assets/audio/characters/{r["file"]}) |')
        rows.append('')
    (ROOT / 'docs/CHARACTER_VOICE_LINES.md').write_text('\n'.join(rows), encoding='utf-8')


if __name__ == '__main__':
    main()
