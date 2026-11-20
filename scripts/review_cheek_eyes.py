"""Render baked UV drawings through the measured native 3D face projection."""
import json,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'tools/python-libs'))
import numpy as np
from PIL import Image, ImageDraw
ROOT=Path(__file__).resolve().parents[1];ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/cheek-eyes'
profiles=json.loads((ASSETS/'cheeks/eyes.json').read_text())
rows=[]
for p in profiles:
    slug=p['slug']
    if not (ASSETS/f'cheeks/{slug}-release-L.png').exists():continue
    d=np.load(ROOT/f'artifacts/face-feature-fix/{slug}-projection.npz');uv=d['uv']
    row=Image.new('RGB',(384*4,410),'#e1e6e9');draw=ImageDraw.Draw(row)
    for col,(tag,path) in enumerate([
        ('neutral',ASSETS/f'{slug}-meshy-basecolor.png'),
        ('hold L',ASSETS/f'cheeks/{slug}-hold-L.png'),
        ('hold R',ASSETS/f"cheeks/{slug}-hold-{'R' if p['hold'][0]!=p['hold'][1] else 'L'}.png"),
        ('release',ASSETS/f'cheeks/{slug}-release-L.png')]):
        a=np.asarray(Image.open(path).convert('RGBA'))[::-1];h,w=a.shape[:2]
        color=a[np.clip((uv[...,1]*h).astype(int),0,h-1),np.clip((uv[...,0]*w).astype(int),0,w-1)].copy()
        color[~d['valid']]=[220,226,230,255]
        row.paste(Image.fromarray(color[::-1]),(col*384,26));draw.text((col*384+10,8),slug+' / '+tag,fill='#182b38')
    row.save(OUT/f'{slug}-drawings.png');rows.append(row)
for i in range(0,len(rows),5):
    sheet=Image.new('RGB',(1536,len(rows[i:i+5])*410))
    for j,row in enumerate(rows[i:i+5]):sheet.paste(row,(0,j*410))
    sheet.save(OUT/f'drawings-{i//5}.png')
print('Drawings reviewed:',len(rows))
