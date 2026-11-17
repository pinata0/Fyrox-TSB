"""Publish Ed's reviewed bodysuit references and rebuild the Tripo packages."""
from pathlib import Path
import hashlib
import json
import shutil
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
INPUTS = ROOT / 'assets/character-inputs'
ART = ROOT / 'artifacts/id-bodysuit-correction'
IMAGES = ART / 'images'


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    generation = read(ART / 'generation.json')
    entries = {entry['view']: entry for entry in generation['entries']}
    IMAGES.mkdir(parents=True, exist_ok=True)
    for view, entry in entries.items():
        shutil.copy2(entry['source'], IMAGES / f'{view}.png')
    write(IMAGES / 'generation.json', generation)
    mounted = IMAGES / 'mounted-front.png'
    mounted_hash = digest(mounted)
    width, height = struct.unpack('>II', mounted.read_bytes()[16:24])
    for directory in ['inputs', 'inputs-masked', 'prepared-preview']:
        shutil.copy2(mounted, INPUTS / directory / 'id.png')
        manifest = INPUTS / directory / 'manifest.json'
        if manifest.exists():
            data = read(manifest)
            for entry in data['characters']:
                if entry['slug'] != 'id':
                    continue
                for stale in ['pixel_box', 'foreground_box']:
                    entry.pop(stale, None)
                entry.update(source=str(mounted), source_sha256=mounted_hash, operation='image_gen bodysuit correction from user costume references; complete mounted assembly; no crop', status='generated_reviewed')
                for key in ['input', 'masked_input']:
                    if key in entry:
                        entry[key] = str(INPUTS / directory / 'id.png')
                for key in ['sha256', 'input_sha256']:
                    if key in entry:
                        entry[key] = mounted_hash
            write(manifest, data)
    checks = read(INPUTS / 'prepared-preview/checks.json')
    for entry in checks:
        if entry['slug'] == 'id':
            entry.update(shape=[height, width, 4], source=str(mounted), correction=generation['correction'])
    write(INPUTS / 'prepared-preview/checks.json', checks)
    plan = read(INPUTS / 'input-plan.json')
    plan['characters'].pop('id', None)
    plan.setdefault('generated_replacements', {})['id'] = dict(path=mounted.relative_to(ROOT).as_posix(), sha256=mounted_hash, reason='Fitted sleeveless bodysuit correction; mounted reference. Tripo standalone input uses images/front.png.')
    write(INPUTS / 'input-plan.json', plan)
    for views in INPUTS.glob('*/id/views'):
        for view in ['front', 'left', 'right', 'back']:
            shutil.copy2(IMAGES / f'mounted-{view}.png', views / f'{view}.png')
        write(views / 'views-report.json', dict(stage='reviewed_bodysuit_correction', source='artifacts/id-bodysuit-correction/generation.json', operation='independently generated and reviewed views', note='User-provided costume references; side/back and unseen footwear are fan-model interpretations.', output_sha256={v: digest(views / f'{v}.png') for v in ['front', 'left', 'right', 'back']}))
    shutil.copy2(IMAGES / 'sheet.png', ROOT / 'assets/turnarounds/id.png')
    data = read(ROOT / 'assets/turnarounds/generation.json')
    for entry in data['entries']:
        if entry['slug'] == 'id':
            entry.update(source=entries['sheet']['source'], prompt=entries['sheet']['prompt'], revision=entries['sheet'].get('revision'), correction=generation['correction'], reference_urls=generation['source_urls'])
    write(ROOT / 'assets/turnarounds/generation.json', data)
    prompts = read(ROOT / 'assets/turnarounds/prompts.json')
    prompts['prompts']['id'] = entries['sheet']['prompt']
    write(ROOT / 'assets/turnarounds/prompts.json', prompts)
    refs = read(ROOT / 'assets/references/manifest.json')
    for entry in refs:
        if entry['slug'] == 'id':
            entry.pop('tracksuit_correction', None)
            entry['outfit_correction'] = dict(authority=generation['authority'], source_urls=generation['source_urls'], images=[dict(path=p, sha256=digest(ROOT / p)) for p in generation['references']], finding=generation['correction'])
    write(ROOT / 'assets/references/manifest.json', refs)
    path = INPUTS / 'tripo-roster/generation-prompts.json'
    # The current Tripo pack owns its generation provenance and open-eye body
    # image. Rebuilding canonical sleepy/mounted art must not revert that input.
    import prepare_tripo_roster
    prepare_tripo_roster.main()
    individual = INPUTS / 'tripo-individual'
    png = individual / '이드-id.png'
    txt = individual / '이드-id-prompt.txt'
    assert 'SLEEVELESS ONE-PIECE BODYSUIT' in txt.read_text(encoding='utf-8')
    assert digest(png) == digest(INPUTS / 'tripo-roster/id/front.png')
    assert digest(mounted) == digest(INPUTS / 'tripo-roster/id/mounted-reference.png')
    print('ID_BODYSUIT_ASSETS_PUBLISHED')


if __name__ == '__main__':
    main()
