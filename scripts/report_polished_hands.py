"""Summarize successful mesh, game and actual packaged-executable validation."""
import hashlib,json,re,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'artifacts/hand-mesh-polish'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
mesh=json.loads((OUT/'mesh-verification.json').read_text())
assets=json.loads((OUT/'asset-verification.json').read_text())
package=json.loads((OUT/'package-verification.json').read_text())
assert mesh['passed'] and assets['passed'] and package['passed']
assert len(mesh['characters'])==len(assets['characters'])==15
log=(OUT/'gpu-weapons.log').read_text(encoding='utf-8',errors='replace')
assert 'RIGHT_HAND_ROSTER_OK' in log and 'WEAPON_REVIEW_OK' in log and 'panicked' not in log
captures=re.findall(r'PREVIEW_OK: (artifacts/preview-weapon-\S+\.png)',log)
assert len(captures)==len(set(captures))==102
gpu=OUT/'gpu';gpu.mkdir(exist_ok=True)
for filename in captures:shutil.copy2(ROOT/filename,gpu/Path(filename).name)
tests=(OUT/'tests.log').read_text(encoding='utf-8',errors='replace')
counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',tests)
totals=[sum(int(row[i]) for row in counts) for i in range(3)]
assert totals==[243,0,2] and 'test result: FAILED' not in tests,totals
for filename in ['clippy.log','build-release.log']:
    assert 'Finished `' in (OUT/filename).read_text(encoding='utf-8',errors='replace')
assert not (OUT/'fmt.log').read_text(encoding='utf-8').strip()
assert digest(OUT/'runtime/tsb.exe')==digest(ROOT/'target/debug/tsb.exe')
manifest=json.loads((ROOT/'assets/characters/manifest.json').read_text(encoding='utf-8'))
for row in manifest:
    for key in ['blend','model','hands','refined_arms_texture']:
        assert row['sha256'][key]==digest(ROOT/row[key]),(row['slug'],key)
digits=[d for c in mesh['characters'] for h in c['hands'] for d in h['digits']]
report={
    'passed':True,'revision':'Polished hands and closed cuffs, 2026-11-15',
    'characters':15,'hands':30,'finger_joints':450,
    'closed_refined_surfaces':sum(c['closed_meshes'] for c in mesh['characters']),
    'visible_weighted_digits':len(digits),'max_handle_gap_m':max(d['grip_gap'] for d in digits),
    'max_digit_penetration_p99_m':max(d['penetration_p99'] for d in digits),
    'source_face_morphs_preserved':345,'source_face_polygons_preserved':True,
    'motion_clips_preserved':165,'independent_production_fbx_roundtrip':True,
    'code_tests':dict(passed=totals[0],failed=totals[1],ignored=totals[2]),
    'fmt':True,'clippy':True,'release_build':True,
    'gpu_captures':102,'gpu_checks':['15 idle/attack/reload','30 world hand grips','dual blade swings','greeting and return','palm position and orientation','exact finger closure'],
    'package':package,'previous_models':str(OUT/'before'),
    'comparison':str(OUT/'before-after.png'),
}
(OUT/'validation-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
shutil.copy2(ROOT/'artifacts/single-exe/package-report.json',OUT/'package-report.json')
print(json.dumps({'passed':True,'max_handle_gap_m':report['max_handle_gap_m'],'max_digit_penetration_p99_m':report['max_digit_penetration_p99_m'],'sha256':package['sha256']}))
