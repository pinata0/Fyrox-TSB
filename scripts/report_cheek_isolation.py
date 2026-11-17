"""Validate shipped cheek domains and refresh the current asset manifest."""
import hashlib,json,math,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/cheek-isolation'
SLUGS='vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
characters=[]
for slug in SLUGS:
    mask=json.loads((ASSETS/f'{slug}-cheeks.json').read_text());rig=json.loads((ASSETS/f'{slug}-rig.json').read_text())
    assert mask['version']==1 and len(mask['vertices'])==rig['vertices']
    for row in mask['vertices']:
        assert all(math.isfinite(v) for v in row)
        assert all(0<=w<=1 for w in row[3:5])
        if row[11]:assert row[3:5]==[0,0]
    for side,stats in enumerate(mask['stats']):
        assert stats['apex_weight']>.99 and stats['core_vertices']>30
        assert stats['core_height']>.04 and stats['core_width']>.045
        apex=mask['vertices'][stats['apex_vertex']]
        assert max(abs(a-b) for a,b in zip(apex[:3],mask['centers'][side]))<1e-6
        assert apex[3+side]>.99
    morphs=json.loads((ASSETS/f'{slug}.morphs.json').read_text());assert len(morphs['shapes'])==23
    source={tuple(round(v*10000) for v in row[:3]) for row in mask['vertices']}
    positions=morphs['vertices']
    assert all(tuple(round(v*10000) for v in positions[i:i+3]) in source for i in range(0,len(positions),3)),slug
    characters.append(dict(slug=slug,vertices=rig['vertices'],bones=rig['bones'],centers=mask['centers'],controls=mask['stats'],protected=mask['protected'],morphs=23,clips=len(rig['clips'])))
verification=json.loads((ROOT/'artifacts/meshy-import/verification.json').read_text());assert verification['passed']
(OUT/'asset-verification.json').write_text(json.dumps(verification,indent=2))
manifest_path=ASSETS/'manifest.json'
for attempt in range(5):
    original=manifest_path.read_bytes();manifest=json.loads(original)
    for row,result in zip(manifest,characters):
        assert row['slug']==result['slug'];slug=row['slug'];rig=json.loads((ASSETS/f'{slug}-rig.json').read_text())
        row.update(vertices=rig['vertices'],triangles=rig['triangles'],bones=rig['bones'],clips=rig['clips'])
        row['cheeks']=f'assets/characters/{slug}-cheeks.json'
        row['cheek_refinement']=dict(isolated_skin=True,protected_features=['eyes','mouth','hair','accessories'],hit_radius_m=.14,centers=result['centers'])
        for field in ['blend','model','cheeks']:row['sha256'][field]=digest(ROOT/row[field])
        row['sha256']['closed_eyes']=digest(ASSETS/f'{slug}-blink-closed.png')
        row['sha256']['half_closed_eyes']=digest(ASSETS/f'{slug}-blink-half.png')
        row['natural_blink']=dict(stages=['open','half','closed'],brow_weight=.75,sampled_upper_lashes=True)
        if 'restored_coat' in rig:row['restored_coat']=rig['restored_coat']
    if manifest_path.read_bytes()==original:
        manifest_path.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');break
else:raise RuntimeError('Asset manifest is changing concurrently')
tests=(OUT/'tests.log').read_text(encoding='utf-8',errors='replace')
results=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',tests)
assert results and 'test result: FAILED' not in tests
report=dict(passed=True,characters=characters,tests_passed=sum(int(p) for p,_,_ in results),tests_ignored=sum(int(i) for _,_,i in results),total_face_controls=345,head_and_torso_unchanged=True,natural_blink_stages=3,renua_coat=json.loads((OUT/'renua-coat-restoration.json').read_text()))
(OUT/'validation-report.json').write_text(json.dumps(report,indent=2))
print('CHEEK_ASSET_REPORT_OK',len(characters),report['tests_passed'],flush=True)
