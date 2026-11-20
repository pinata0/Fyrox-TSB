"""Validate all drawn eye atlases and their archived original source evidence."""
import hashlib,json,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'tools/python-libs'))
import numpy as np
from PIL import Image
ROOT=Path(__file__).resolve().parents[1];ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/cheek-eyes'
profiles=json.loads((ASSETS/'cheeks/eyes.json').read_text())
motions=json.loads((ASSETS/'cheeks/motions.json').read_text())
sources=json.loads((ROOT/'artifacts/cheek-interaction/research/sources.json').read_text(encoding='utf8'))
assert len(profiles)==len(motions)==len(sources)==15
report=[]
for p,m,s in zip(profiles,motions,sources):
    slug=p['slug'];assert slug==m['slug']==s['slug']
    source=ROOT/f'artifacts/cheek-interaction/research/{slug}.skel'
    assert hashlib.sha256(source.read_bytes()).hexdigest()==s['sha256']
    research=json.loads((OUT/f'reference/{slug}/eyes.json').read_text())
    for phase,name in [('hold','Touch_Idle'),('release','Touch_End')]:
        assert abs(research['clips'][name]['duration']-m[phase]['duration'])<.00001
    d=np.load(ROOT/f'artifacts/face-feature-fix/{slug}-projection.npz')
    face=json.loads((ASSETS/f'{slug}-rig.json').read_text())['face']
    x,z=d['pos'][...,0],d['pos'][...,2];base=d['base'][...,:3]
    brow=d['valid']&(abs(abs(x-face['center_x'])-face['eye_x'])<face['eye_x']*.9)
    brow&=(z>face['eye_y']+face['eye_half_height']*1.75)&(z<face['eye_y']+face['eye_half_height']*2.5)&(base.max(2)<.45)
    files=[]
    for phase in ['hold','release']:
        for side in (['L','R'] if p[phase][0]!=p[phase][1] else ['L']):
            path=ASSETS/f'cheeks/{slug}-{phase}-{side}.png'
            with Image.open(path) as im:im.verify()
            im=Image.open(path).convert('RGBA');assert im.size==(2048,2048)
            pixels=np.asarray(im);assert pixels[:,:,3].min()==255
            uv=d['uv'];sample=pixels[::-1][np.clip((uv[...,1]*2048).astype(int),0,2047),np.clip((uv[...,0]*2048).astype(int),0,2047),:3]/255
            lost=int((brow&(sample.max(2)>.6)).sum());assert lost==0,(slug,phase,'forehead brow was erased',lost)
            ink=[]
            for sign in ([-1] if slug=='aya' else [-1,1]):
                eye=d['valid']&(abs(x-(face['center_x']+sign*face['eye_x']))<face['eye_x']*1.1)
                eye&=(z>face['eye_y']-face['eye_half_height']*.65)&(z<face['eye_y']+face['eye_half_height']*1.1)
                count=int((eye&(sample.max(2)<.45)).sum());assert count>30,(slug,phase,sign,'missing dark eyelash')
                ink.append(count)
            if slug=='shion' and phase=='release':
                for sign in [-1,1]:
                    iris=d['valid']&(abs(x-(face['center_x']+sign*face['eye_x']))<face['eye_x']*.5)
                    iris&=abs(z-face['eye_y'])<face['eye_half_height']*.7
                    def eye_colour(a):
                        if sign<0:return (a[:,:,1]>a[:,:,0]+.12)&(a[:,:,2]>a[:,:,0]+.12)
                        return (a[:,:,0]>a[:,:,1]+.14)&(a[:,:,2]>a[:,:,1]+.14)
                    assert (iris&eye_colour(base)).sum()>100,'neutral heterochromia reference'
                    assert (iris&eye_colour(sample)).sum()>100,'Shion iris colours must stay on their original sides'
            files.append({'file':str(path.relative_to(ROOT)),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'brow_pixels_preserved':int(brow.sum()),'eyelash_pixels':ink})
    report.append({'slug':slug,'source':s['viewer'],'source_sha256':s['sha256'],'hold':p['hold'],'release':p['release'],'files':files})
(OUT/'asset-verification.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
print('CHEEK_EYES_ASSETS_OK',len(report),'heroes',sum(len(r['files']) for r in report),'atlases')
