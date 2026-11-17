"""Run the production renderer with the candidate character assets."""
import os,re,shutil,subprocess,sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/blink-surface-fix'
stage=OUT/'runtime-assets'
stage.mkdir(exist_ok=True)
for source in (ROOT/'assets/characters').iterdir():
    if not source.is_file() or source.suffix not in ['.png','.fbx','.json']:continue
    candidate=OUT/'candidate'/source.name
    actual=candidate if candidate.exists() else source
    dest=stage/source.name
    if dest.exists():dest.unlink()
    shutil.copy2(actual,dest)
env=os.environ.copy();env['TSB_CHARACTER_ASSET_DIR']=str(stage)
label=sys.argv[1] if len(sys.argv)>1 else 'focus'
with (OUT/f'gpu-{label}.log').open('w',encoding='utf-8') as log:
    result=subprocess.run([str(next(OUT.glob('tsb-review*.exe'))),'--selection-review','--blink-review','--hud-review-small'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW,timeout=900)
text=(OUT/f'gpu-{label}.log').read_text(encoding='utf-8',errors='replace')
assert result.returncode==0 and 'BLINK_REVIEW_OK' in text,(result.returncode,text[-2000:])
dest=OUT/f'gpu-{label}';dest.mkdir(exist_ok=True)
for name in re.findall(r'PREVIEW_OK: (.+\.png)',text):
    source=ROOT/name
    if 'preview-blink-' in source.name:shutil.copy2(source,dest/source.name)
print('BLINK_GPU_REVIEW_OK',label,flush=True)
