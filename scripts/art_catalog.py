"""Rebuild the local art catalog and provenance hashes (standard Python only)."""
import hashlib
import html
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
refs=json.loads((ROOT/'assets/references/manifest.json').read_text(encoding='utf-8-sig'))
models=json.loads((ROOT/'assets/characters/manifest.json').read_text(encoding='utf-8'))
generated=json.loads((ROOT/'assets/turnarounds/generation.json').read_text(encoding='utf-8-sig'))
assert len(refs)==len(models)==len(generated['entries'])==15
cards=[]
for ref,model in zip(refs,models):
    slug=ref['slug']; assert slug==model['slug']
    for entry in ref['files']:entry['kind']='2D texture atlas, reference only'
    screenshot=ROOT/'assets/references'/f'{slug}-fullbody.png'
    assert screenshot.is_file()
    ref['fullbody_capture']={'path':screenshot.relative_to(ROOT).as_posix(),'url':ref['motion_viewer'],'sha256':hashlib.sha256(screenshot.read_bytes()).hexdigest()}
    model['sha256']={}
    for key in ['blend','model','guide'] + (['portrait'] if 'portrait' in model else []):
        p=ROOT/model[key]; assert p.is_file()
        model['sha256'][key]=hashlib.sha256(p.read_bytes()).hexdigest()
    if model.get('textures'):
        model['texture_sha256']={path:hashlib.sha256((ROOT/path).read_bytes()).hexdigest() for path in model['textures']}
    if model.get('weapon_model'):
        model['sha256']['weapon_model']=hashlib.sha256((ROOT/model['weapon_model']).read_bytes()).hexdigest()
    name=html.escape(ref['name'])
    cards.append(f'''<article><h2>{ref['id']:02d} · {name}</h2><img class="model" src="{slug}-preview.png" alt="{name} Blender 모델">
    <nav><a href="{slug}.blend">Blender 원본</a><a href="{slug}.fbx">게임용 FBX</a><a href="../turnarounds/{slug}.png">삼면도 PNG</a><a href="{ref['motion_viewer']}">원작 모션 자료</a></nav>
    <details><summary>삼면도 보기</summary><img src="../turnarounds/{slug}.png" alt="{name} 정면 측면 후면"></details>
    <p>{model['triangles']:,} triangles · {model['bones']} bones · {len(model['clips'])} motions</p></article>''')
(ROOT/'assets/references/manifest.json').write_text(json.dumps(refs,ensure_ascii=False,indent=2),encoding='utf-8')
(ROOT/'assets/characters/manifest.json').write_text(json.dumps(models,ensure_ascii=False,indent=2),encoding='utf-8')
(ROOT/'assets/characters/index.html').write_text('''<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>TSB 캐릭터 아트</title>
<link rel="icon" href="../ui/icons/favicon.ico" sizes="16x16 32x32 48x48">
<style>body{margin:0;padding:40px;background:#f1eee8;color:#243449;font:16px system-ui,sans-serif}h1{font-size:36px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:24px}article{background:white;border-radius:18px;padding:20px;box-shadow:0 4px 18px #263a4c10}h2{font-size:20px}img{max-width:100%}.model{width:100%;background:#edf0f4;border-radius:12px}nav{display:flex;gap:10px;flex-wrap:wrap;margin:15px 0}a{color:#25665d}p{line-height:1.6}summary{cursor:pointer}</style>
<h1>Trickcal : Shooting Balddagu</h1><p>15명 캐릭터 아트 · 삼면도 / Blender / FBX / 원작 자료<br>G 춤 전환 · 마우스 360도 관찰 · 휠 줌 · 이동/공격 시 1인칭 복귀</p>
<p>원작 캐릭터 디자인: EPID Games. 새로 제작한 팬게임용 MVP 3D 해석이며 측면·후면 일부와 춤 동작은 제작 과정에서 구성했습니다.</p><main>'''+''.join(cards)+'''</main><p>자료와 프롬프트: assets/references/manifest.json · assets/turnarounds/generation.json<br>제작·검증 기록: docs/CHARACTER_ART.md · docs/WORK_LOG.md</p></html>''',encoding='utf-8')
print('ART_CATALOG_OK: 15 guides, captures, blends, FBXs and provenance hashes')
