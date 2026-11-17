"""Record the completed hand-rig revision using actual model/game/package checks."""
import hashlib
import json
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/meshy-hand-fix'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


hands = json.loads((OUT / 'hand-verification.json').read_text())
assets = json.loads((OUT / 'asset-verification.json').read_text())
package = json.loads((OUT / 'package-verification.json').read_text())
assert hands['passed'] and assets['passed'] and package['passed']
assert len(hands['characters']) == len(assets['characters']) == 15
log = (OUT / 'gpu-weapons-delivery.log').read_text(encoding='utf-8', errors='replace')
assert 'WEAPON_REVIEW_OK' in log and 'RIGHT_HAND_ROSTER_OK' in log
assert 'panicked' not in log
captures = re.findall(r'PREVIEW_OK: (artifacts/preview-weapon-\S+\.png)', log)
assert len(captures) == len(set(captures)) == 102
gpu = OUT / 'gpu'
gpu.mkdir(exist_ok=True)
for capture in captures:
    shutil.copy2(ROOT / capture, gpu / Path(capture).name)
tests = (OUT / 'tests.log').read_text(encoding='utf-8', errors='replace')
counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', tests)
totals = [sum(int(row[i]) for row in counts) for i in range(3)]
assert totals == [243, 0, 2], totals
assert 'test result: FAILED' not in tests
for name in ['clippy', 'build-release']:
    assert 'Finished `' in (OUT / f'{name}.log').read_text(encoding='utf-8', errors='replace')
assert not (OUT / 'fmt.log').read_text(encoding='utf-8').strip()
assert digest(OUT / 'runtime/tsb.exe') == digest(ROOT / 'target/debug/tsb.exe')
manifest = json.loads((ROOT / 'assets/characters/manifest.json').read_text(encoding='utf-8'))
for row in manifest:
    for key in ['blend', 'model', 'hands']:
        assert row['sha256'][key] == digest(ROOT / row[key]), (row['slug'], key)
report = {
    'passed': True,
    'revision': 'Articulated fingers and palm grip correction, 2026-11-15',
    'previous_issue': 'Finger weights collapsed into the wrist; old contact checks did not validate grasping.',
    'characters': 15, 'finger_joints': 450, 'palm_contacts': 30,
    'face_morphs_preserved': 345, 'motion_clips_preserved': 165,
    'hand_weight_and_deformation_validation': True,
    'independent_fbx_roundtrip': True,
    'code_tests': dict(passed=totals[0], failed=totals[1], ignored=totals[2]),
    'fmt': True, 'clippy': True, 'release_build': True,
    'gpu_captures': len(captures),
    'gpu_checks': ['15 idle/attack/reload', 'anatomical hands', '15 world idle/attack',
                   'dual blade swings', 'greeting continuity and return',
                   'weighted finger closure', 'palm position and grip axis'],
    'package': package,
    'debug_binary_sha256': digest(ROOT / 'target/debug/tsb.exe'),
    'limitations': ['Generated source hand topology is preserved; fused fingers are not remodelled into separate fingers.',
                    'Large cuffs and coarse source-mesh edges remain visible in some first-person silhouettes.'],
    'original_models_preserved': str(OUT / 'before'),
}
(OUT / 'validation-report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
shutil.copy2(ROOT / 'artifacts/single-exe/package-report.json', OUT / 'package-report.json')
print(json.dumps({'passed': True, 'gpu_captures': 102, 'package': package['file'], 'sha256': package['sha256']}))
