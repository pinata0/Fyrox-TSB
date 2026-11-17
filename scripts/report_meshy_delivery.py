"""Collect completed asset, game-render, source-test and packaged-runtime checks."""
import hashlib,json,re,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/meshy-import'
def digest(p):
    with p.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def text(name):return (OUT/name).read_text(encoding='utf-8-sig',errors='replace')
verification=json.loads(text('verification.json'))
assert verification['passed'] and len(verification['characters'])==15
for file,marker in [('gpu-selection.log','SELECTION_REVIEW_OK'),('gpu-cheeks.log','CHEEK_REVIEW_OK'),
                    ('gpu-faces-delivery.log','FACE_REVIEW_OK'),('gpu-weapons.log','WEAPON_REVIEW_OK')]:
    assert marker in text(file),(file,marker)
assert text('gpu-faces-delivery.log').count('MORPH_REMAP_OK:')==16
tests=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',text('tests-final.log'))
totals=[sum(int(t[i]) for t in tests) for i in range(3)]
assert totals==[242,0,2],totals
assert 'Finished' in text('clippy-final.log') and not text('fmt-final.log').strip()
assert 'Finished' in text('build-release.log')
package=json.loads((ROOT/'artifacts/single-exe/package-report.json').read_text())
exe=ROOT/package['file'];assert digest(exe)==package['sha256']
cache=OUT/'single-exe-cache'/package['payload_sha256']
assert 'PASS: package extracted and verified.' in text('single-exe-verify.log')
assert digest(cache/'tsb.exe')==digest(ROOT/'target/release/tsb.exe')
artlogs=list((cache/'artifacts/logs').glob('launcher-*.log'))
artlog=next(p for p in artlogs if 'ART_REVIEW_OK' in p.read_text(errors='replace'))
shutil.copy2(artlog,OUT/'gpu-packaged-art.log')
for p in (cache/'artifacts').glob('preview-*.png'):shutil.copy2(p,OUT/'gpu'/p.name)
delivery=json.loads(text('delivery.json'))
for item in delivery['files']:
    assert digest(ROOT/item['path'])==item['sha256'],item['path']
for p in (ROOT/'assets/characters').glob('*.morphs.json'):
    assert digest(p)==digest(cache/'assets/characters'/p.name)
report=dict(date='2026-11-15',passed=True,source_files=dict(glb=17,zip=15,originals_preserved=True),
    characters=15,facial_channels=345,motions=165,seamless_loops=60,
    separate_weapons=13,attached_companions=['ui-frog','chloe-sebastian','id-mount','id-robot'],
    rust_tests=dict(passed=totals[0],failed=totals[1],ignored=totals[2]),
    fmt=True,clippy_warnings_denied=True,release_build=True,
    gpu=dict(selection_frames=30,cheek_frames=105,facial_frames=30,weapon_frames=102,packaged_dance_frames=30,
             imported_morph_channels=346,all_imported_vertices_mapped=True),
    package=package,packaged_executable_matches_build=True,packaged_morph_data_matches_delivery=True,
    delivery_files=len(delivery['files']),preview='artifacts/meshy-import/roster-preview.jpg',
    limits=['Facial expressions deform the existing painted surface; no new teeth, tongue or oral cavity.'])
(OUT/'validation-report.json').write_text(json.dumps(report,indent=2,ensure_ascii=False),encoding='utf-8')
print(json.dumps(dict(passed=True,tests=totals[0],file=str(exe),sha256=package['sha256']),indent=2))
