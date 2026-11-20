"""Import the four requested Trickcal OST tracks (requires ffmpeg on PATH)."""
import hashlib
import html
import json
from pathlib import Path
import re
import subprocess
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
ALBUM = 'https://downloads.khinsider.com/game-soundtracks/album/trickcal-revive-soundtrack-collection-2024'
TRACKS = [
    (1, 'Trickal Main Title', 'main_title', 'Main menu'),
    (2, 'Day Life', 'day_life', 'Battle lobby'),
    (3, 'Lounge', 'lounge', 'Hero selection'),
    (13, 'Adventure Begins', 'adventure_begins', 'In game'),
]


def fetch(url):
    request = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
    with urllib.request.urlopen(request, timeout=90) as response:
        return response.read()


def main():
    cache = ROOT / 'artifacts/music-sources'
    dest = ROOT / 'assets/audio/music'
    cache.mkdir(parents=True, exist_ok=True)
    dest.mkdir(parents=True, exist_ok=True)
    sources = []
    for number, title, stem, screen in TRACKS:
        filename = f'{number:02d}. {title}.mp3'
        page = ALBUM + '/' + urllib.parse.quote(urllib.parse.quote(filename))
        markup = fetch(page).decode('utf-8')
        links = [html.unescape(link) for link in re.findall(r'href="([^"]+)"', markup)]
        source_url = next(link for link in links if link.startswith('https://') and link.endswith('.flac'))
        source = cache / f'{stem}.flac'
        if not source.exists():
            data = fetch(source_url)
            if not data.startswith(b'fLaC'):
                raise ValueError(f'Not a FLAC audio file: {title}')
            source.write_bytes(data)
        output = dest / f'{stem}.ogg'
        subprocess.run([
            'ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-i', str(source),
            '-map_metadata', '-1', '-vn', '-af', 'loudnorm=I=-18:TP=-1.5:LRA=11',
            '-ar', '44100', '-ac', '2', '-c:a', 'libvorbis', '-q:a', '5',
            '-metadata', f'title={title}', '-metadata', 'artist=Warak', str(output),
        ], check=True)
        sources.append(dict(title=title, screen=screen, file=output.name, page=page,
                            source_url=source_url,
                            source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
                            sha256=hashlib.sha256(output.read_bytes()).hexdigest(),
                            bytes=output.stat().st_size))
        print(f'{title}: {output.stat().st_size} bytes', flush=True)
    (dest / 'sources.json').write_text(json.dumps({
        'album': 'Trickcal Revive Soundtrack Collection (2024)',
        'composer': 'Warak', 'publisher': 'EPID Games', 'album_url': ALBUM,
        'rights': 'Original game soundtrack; not CC0. No redistribution license is granted by this manifest.',
        'conversion': 'Source FLAC -> stereo Vorbis quality 5, 44100 Hz; loudnorm -18 LUFS, -1.5 dBTP, LRA 11. Full tracks, no trimming.',
        'tracks': sources,
    }, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
