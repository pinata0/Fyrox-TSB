"""Publish the verified stage atomically, without writing through existing hard links."""
import hashlib
import json
import shutil
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];WORK=ROOT/'artifacts/lowpoly-import'
STAGE=WORK/'characters';BACK=WORK/'before';DEST=ROOT/'assets/characters'

def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

if '--verify' in sys.argv:
    result=json.loads((WORK/'publish-plan.json').read_text())
    for row in result['files']:
        assert digest(DEST/row['path'])==row['sha256'],row['path']
    (WORK/'published.json').write_text(json.dumps(result,indent=2))
    print('LOWPOLY_PUBLISHED',len(result['files']),'files',flush=True)
    raise SystemExit(0)

validation=json.loads((WORK/'rig-validation.json').read_text())
assert len(validation)==15 and len({r['slug'] for r in validation})==15
records=json.loads((STAGE/'manifest.json').read_text())
for row in records:
    for key in row['sha256']:
        if key not in row:continue
        source=ROOT/row[key]
        try:source=STAGE/source.relative_to(DEST)
        except ValueError:pass
        if source.is_file():row['sha256'][key]=digest(source)
(STAGE/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
changed=[]
for source in sorted(STAGE.rglob('*')):
    if not source.is_file() or source.suffix=='.blend1':continue
    relative=source.relative_to(STAGE);target=DEST/relative;backup=BACK/relative
    assert target.resolve().is_relative_to(DEST.resolve())
    sha=digest(source)
    if backup.is_file() and digest(backup)==sha:continue
    assert not target.exists() or digest(target)==sha or (backup.is_file() and digest(target)==digest(backup)),(relative,'production changed since backup')
    changed.append((source,target,relative,sha))
pending=[]
for source,target,relative,sha in changed:
    if target.is_file() and digest(target)==sha:continue
    target.parent.mkdir(parents=True,exist_ok=True)
    temporary=target.with_name(target.name+'.lowpoly-tmp')
    if temporary.exists():assert digest(temporary)==sha,temporary
    else:shutil.copy2(source,temporary)
    assert digest(temporary)==sha
    pending.append(dict(source=str(temporary),target=str(target)))
result=dict(files=[dict(path=rel.as_posix(),sha256=sha) for _,_,rel,sha in changed],characters=validation,
            source_manifest='assets/character-sources/lowpoly/manifest.json',backup=str(BACK))
(WORK/'publish-plan.json').write_text(json.dumps(result,indent=2))
(WORK/'publish-pending.json').write_text(json.dumps(pending,indent=2))
print('LOWPOLY_PUBLICATION_PREPARED',len(pending),'files remaining',flush=True)
