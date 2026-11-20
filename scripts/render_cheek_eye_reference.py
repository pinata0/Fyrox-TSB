"""Render original evaluated Spine triangles, with consistent face crops per hero."""
import sys, json, os
os.environ['OPENBLAS_NUM_THREADS']='1'
os.environ['OMP_NUM_THREADS']='1'
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2]/'tools/python-libs'))
import numpy as np
from PIL import Image, ImageDraw
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/cheek-eyes/reference'
sources=json.loads((ROOT/'artifacts/cheek-interaction/research/sources.json').read_text(encoding='utf8'))
rows=[]
for source in sources:
    slug=source['slug'];folder=OUT/slug
    if (folder/'contact.png').exists():
        rows.append(Image.open(folder/'contact.png').convert('RGB'));continue
    frames=json.loads((folder/'frames.json').read_text());atlases={}
    for f in frames:
        for m in f['meshes']:
            if m['page'] in atlases:continue
            a=Image.open(folder/m['page']).convert('RGBA');p=np.asarray(a).astype(float)
            p[:,:,:3]=np.minimum(255,p[:,:,:3]/np.maximum(p[:,:,3:4]/255,1/255))
            atlases[m['page']]=Image.fromarray(p.astype('uint8'))
    eyepts=np.concatenate([np.array(m['vertices']).reshape(-1,2) for m in frames[0]['meshes'] if 'eye' in m['name'].lower() and not any(n in m['name'].lower() for n in ['drone','seba','teddy'])])
    lo=eyepts.min(0);hi=eyepts.max(0);center=(lo+hi)/2
    scale=235/max(hi[0]-lo[0],hi[1]-lo[1]*1.7)
    row=Image.new('RGB',(300*5,280),'#e1e6e9');draw=ImageDraw.Draw(row)
    for col,index in enumerate([0,2,4,7,9]):
        f=frames[index];canvas=Image.new('RGBA',(300,245))
        for m in f['meshes']:
            atlas=atlases[m['page']]
            p=(np.array(m['vertices']).reshape(-1,2)-center)*[scale,-scale]+[150,100]
            uv=np.array(m['uvs']).reshape(-1,2)*atlas.size
            for tri in np.array(m['triangles']).reshape(-1,3):
                dst=p[tri];src=uv[tri];low=np.maximum(0,np.floor(dst.min(0)).astype(int));high=np.minimum(canvas.size,np.ceil(dst.max(0)).astype(int)+1);size=tuple(high-low)
                if min(size)<=0:continue
                local=dst-low;mat=np.column_stack((local,np.ones(3)))
                if abs(np.linalg.det(mat))<1e-6:continue
                part=atlas.transform(size,Image.Transform.AFFINE,np.linalg.solve(mat,src).T.ravel(),Image.Resampling.BILINEAR)
                mask=Image.new('L',size);ImageDraw.Draw(mask).polygon([tuple(q) for q in local],fill=255)
                part.putalpha(Image.fromarray((np.minimum(np.asarray(part.getchannel('A')),np.asarray(mask))*m['alpha']).astype('uint8')))
                canvas.alpha_composite(part,tuple(low))
        row.paste(canvas,(col*300,30),canvas);draw.text((col*300+10,8),f"{slug}: {f['clip']} {f['phase']:.0%}",fill='#182b38')
    row.save(folder/'contact.png');rows.append(row);print(slug,flush=True)
for page in range(3):
    sheet=Image.new('RGB',(1500,1400))
    for i,row in enumerate(rows[page*5:page*5+5]):sheet.paste(row,(0,i*280))
    sheet.save(OUT/f'original-eyes-{page}.png')
