"""Publish Aya's reviewed ankle-length twin tails and parasol to active assets."""
from pathlib import Path
import hashlib
import json
import shutil
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
INPUTS = ROOT / 'assets/character-inputs'
RECORD = ROOT / 'artifacts/aya-twintail-correction/generation.json'


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    generation = read(RECORD)
    entries = {entry['view']: entry for entry in generation['entries']}
    source_dir = ROOT / 'artifacts/aya-twintail-correction/images'
    source_dir.mkdir(parents=True, exist_ok=True)
    for view, entry in entries.items():
        shutil.copy2(entry['source'], source_dir / f'{view}.png')
    write(source_dir / 'generation.json', generation)
    front = source_dir / 'front.png'
    front_hash = digest(front)
    width, height = struct.unpack('>II', front.read_bytes()[16:24])
    for directory in ['inputs', 'inputs-masked', 'prepared-preview']:
        shutil.copy2(front, INPUTS / directory / 'aya.png')
        manifest = INPUTS / directory / 'manifest.json'
        if manifest.exists():
            data = read(manifest)
            for entry in data['characters']:
                if entry['slug'] != 'aya':
                    continue
                for stale in ['pixel_box', 'foreground_box']:
                    entry.pop(stale, None)
                entry.update(source=str(front), source_sha256=front_hash, operation='image_gen ankle-length twin-tail correction; no crop', status='generated_reviewed')
                for key in ['input', 'masked_input']:
                    if key in entry:
                        entry[key] = str(INPUTS / directory / 'aya.png')
                for key in ['sha256', 'input_sha256']:
                    if key in entry:
                        entry[key] = front_hash
            write(manifest, data)
    checks = read(INPUTS / 'prepared-preview/checks.json')
    for entry in checks:
        if entry['slug'] == 'aya':
            entry.update(shape=[height, width, 4], source=str(front), correction='held flower parasol; two ankle-length low white twin tails')
    write(INPUTS / 'prepared-preview/checks.json', checks)
    plan = read(INPUTS / 'input-plan.json')
    plan['characters'].pop('aya', None)
    plan.setdefault('generated_replacements', {})['aya'] = dict(path='artifacts/aya-twintail-correction/images/front.png', sha256=front_hash, reason='Ankle-length white twin tails and held flower parasol; use this file directly.')
    write(INPUTS / 'input-plan.json', plan)
    for batch in ['20261112-turnaround-direct', '20261112-turnaround-corrected']:
        views = INPUTS / batch / 'aya/views'
        for view in ['front', 'left', 'right', 'back']:
            shutil.copy2(source_dir / f'{view}.png', views / f'{view}.png')
        write(views / 'views-report.json', dict(stage='reviewed_twintail_and_parasol_correction', source='artifacts/aya-twintail-correction/images/generation.json', operation='independently authored views; no mirrored side', note='Two ankle-length low twin tails in every view. Side and back are fan-model interpretations.', output_sha256={v:digest(views/f'{v}.png') for v in ['front','left','right','back']}))
    shutil.copy2(source_dir / 'sheet.png', ROOT / 'assets/turnarounds/aya.png')
    turnaround = ROOT / 'assets/turnarounds/generation.json'
    data = read(turnaround)
    for entry in data['entries']:
        if entry['slug'] == 'aya':
            entry.update(source=entries['sheet']['source'], prompt=entries['sheet']['prompt'], correction='Two ankle-length low white twin tails, separate from the held flower-petal parasol and kimono.', reference_url=generation['source_url'])
    write(turnaround, data)
    prompts_path = ROOT / 'assets/turnarounds/prompts.json'
    prompts = read(prompts_path)
    prompts['prompts']['aya'] = entries['sheet']['prompt']
    write(prompts_path, prompts)
    references_path = ROOT / 'assets/references/manifest.json'
    references = read(references_path)
    for entry in references:
        if entry['slug'] == 'aya':
            entry['parasol_correction'] = dict(source_url=generation['source_url'], image='assets/references/aya-parasol-reference.webp', sha256=digest(ROOT/'assets/references/aya-parasol-reference.webp'), finding=generation['correction'])
    write(references_path, references)
    # Record the actual prompt for the corrected Tripo input, not its superseded hair-only description.
    prompt_path = INPUTS / 'tripo-roster/generation-prompts.json'
    prompt_data = read(prompt_path)
    prompt_data['entries'] = [e for e in prompt_data['entries'] if e['slug'] != 'aya']
    prompt_data['entries'].append(dict(slug='aya', input='assets/references/aya-parasol-reference.webp', output='aya/front.png', transparent_background=True, prompt=entries['front']['prompt'], source=entries['front']['source']))
    write(prompt_path, prompt_data)
    import prepare_tripo_roster
    prepare_tripo_roster.main()
    # Keep the per-character copies and ZIP in step with the main pack.
    individual = INPUTS / 'tripo-individual'
    png = individual / '아야-aya.png'
    txt = individual / '아야-aya-prompt.txt'
    shutil.copy2(front, png)
    shutil.copy2(INPUTS / 'tripo-roster/aya/prompt.txt', txt)
    with zipfile.ZipFile(individual / '아야-aya.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.write(png, png.name); archive.write(txt, txt.name)
    with zipfile.ZipFile(individual / '아야-aya.zip') as archive:
        assert archive.testzip() is None
    assert all(term in txt.read_text(encoding='utf-8') for term in ['PARASOL','ANKLE-LENGTH','TWIN TAILS'])
    assert all(digest(INPUTS / p) == front_hash for p in ['inputs/aya.png','inputs-masked/aya.png','prepared-preview/aya.png','tripo-roster/aya/front.png','tripo-individual/아야-aya.png'])
    print('AYA_REFERENCE_CORRECTION_OK')


if __name__ == '__main__':
    main()
