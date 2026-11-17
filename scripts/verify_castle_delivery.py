"""Record reproducible artifact and runtime evidence for the concept-room delivery."""
import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path

root=Path(__file__).resolve().parents[1]
out=root/'artifacts/castle-concept-v3'
asset=root/'assets/maps/fairy-kingdom-castle'
def read(p):return p.read_text(encoding='utf-8-sig',errors='replace')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
layout=json.loads(read(asset/'layout.json'))
stats=json.loads(read(asset/'build-report.json'))
catalog=json.loads(read(asset/'object-catalog.json'))
refs=root/'assets/references/fairy-kingdom-castle/concept'
manifest=json.loads(read(refs/'manifest.json'))
for ref in manifest:assert sha(refs/ref['file'])==ref['sha256'],ref['file']
assert {ref for p in catalog for ref in p['reference']}==set(range(1,10))
assert len(layout['blocks'])==stats['collision_blocks']
game=read(out/'game-review.log')
build=read(out/'build-delivery.log')
tests=read(out/'tests.log')
assert 'CASTLE_MESH_QA_OK' in build and 'CASTLE_MODEL_OK' in build
assert f"{stats['export_chunks']} mesh chunks / {len(layout['lights'])} room lights" in game
assert 'MAP_REVIEW_OK: fairy-kingdom-castle / 19 authored views' in game
assert not re.search(r'\[ERROR\]|panicked at|Resource loading failed',game)
results=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',tests)
assert len(results)>=12 and all(int(failed)==0 for _,failed,_ in results)
assert 'error:' not in tests and 'FAILED' not in tests
for name in ['clippy.log','desktop-build.log','server-build.log']:
    log=read(out/name);assert 'Finished' in log and 'error:' not in log,name
assert not read(out/'fmt.log').strip()
assert 'CASTLE_RENDER_OK' in read(out/'build-final.log')
assert 'CASTLE_CONCEPT_RENDER_OK' in read(out/'render-delivery.log')
paths=[asset/name for name in ['layout.json','fairy-kingdom-castle.blend','fairy-kingdom-castle.fbx','object-catalog.json','build-report.json','preview.png','window-emission.png']]
paths += [root/'scripts'/name for name in ['castle_layout.py','castle_concept_layout.py','castle_concept_assets.py','castle_scenery.py','build_castle.py','render_castle_plan.ps1','render_castle_concepts.py','review_castle_concepts.ps1']]
paths += [root/name for name in ['src/level.rs','src/presentation/castle.rs','src/presentation/map_review.rs','tests/castle.rs','docs/FAIRY_KINGDOM_CASTLE.md']]
paths += sorted(out.glob('game-*.png'))
paths += sorted((root/'artifacts/fairy-kingdom-castle').glob('concept-*.png'))
paths += [out/name for name in ['tsb-concept-review.exe','tests.log','routes.log','clippy.log','desktop-build.log','server-build.log','fmt.log','build-delivery.log','render-delivery.log','game-review.log']]
paths.append(root/'scripts/verify_castle_delivery.py')
report=dict(verified_utc=datetime.now(timezone.utc).isoformat(),slug=layout['slug'],revision=layout['revision'],
    model=stats,reference_images=len(manifest),reference_hashes_verified=True,object_instances=len(catalog),
    object_types=len({p['kind'] for p in catalog}),spaces=len(layout['spaces']),
    routes=len(layout['routes']),room_lights=len(layout['lights']),
    tests_passed=sum(int(p) for p,_,_ in results),tests_ignored=sum(int(i) for _,_,i in results),
    gpu_views=19,blender_views=13,mesh_qa='No zero-area triangles; normals and UVs generated',
    checks=['all-targets','castle and multilevel routes','fmt','clippy -D warnings','client/server build','Blender generation/render','Fyrox 19-view capture'],
    unchanged_rules=['Own active spawn invulnerability','AI target exclusion and immediate chase release'],
    limitations=['Concept-art interpretation, not extracted original game meshes or exact dimensions','Human 6v6 balance not playtested','Stylized materials and window fills, not original painted textures or volumetric lighting'],
    files={str(p.relative_to(root)).replace('\\','/'):sha(p) for p in paths})
payload=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
(out/'validation.json').write_text(payload,encoding='utf-8')
(root/'artifacts/fairy-kingdom-castle/validation.json').write_text(payload,encoding='utf-8')
print('CASTLE_DELIVERY_OK',report['tests_passed'],'tests;',report['object_instances'],'props;',stats['export_chunks'],'chunks')
