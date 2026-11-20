"""Offline orthographic rendering of evaluated Spine mesh triangles for review."""
import json
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/renua-skills/reference'
frames=json.loads((OUT/'frames.json').read_text())
atlas=Image.open(OUT/'RenewaAwaken.png').convert('RGBA')
pixels=np.asarray(atlas).astype(float); alpha=pixels[:,:,3:4]/255
pixels[:,:,:3]=np.minimum(255,pixels[:,:,:3]/np.maximum(alpha,1/255))
atlas=Image.fromarray(pixels.astype('uint8'))
allpts=np.concatenate([np.array(m['vertices']).reshape(-1,2) for f in frames for m in f['meshes']])
lo=allpts.min(0);hi=allpts.max(0);scale=min(350/(hi-lo)[0],420/(hi-lo)[1])
sheet=Image.new('RGB',(400*8,480*4),'#d8e0e5');draw=ImageDraw.Draw(sheet)
for j,f in enumerate(frames):
    canvas=Image.new('RGBA',(400,450))
    for mesh in f['meshes']:
        p=np.array(mesh['vertices']).reshape(-1,2);p=(p-(lo+hi)/2)*[scale,-scale]+[200,225]
        uv=np.array(mesh['uvs']).reshape(-1,2)*atlas.size
        for tri in np.array(mesh['triangles']).reshape(-1,3):
            dst=p[tri];src=uv[tri]
            low=np.maximum(0,np.floor(dst.min(0)).astype(int));high=np.minimum(canvas.size,np.ceil(dst.max(0)).astype(int)+1)
            size=tuple(high-low)
            if min(size)<=0:continue
            local=dst-low
            mat=np.column_stack((local,np.ones(3)))
            if abs(np.linalg.det(mat))<1e-6:continue
            coeff=np.linalg.solve(mat,src).T.ravel()
            part=atlas.transform(size,Image.Transform.AFFINE,coeff,Image.Resampling.BILINEAR)
            mask=Image.new('L',size);ImageDraw.Draw(mask).polygon([tuple(q) for q in local],fill=255)
            a=np.minimum(np.asarray(part.getchannel('A')),np.asarray(mask))
            part.putalpha(Image.fromarray((a*mesh['alpha']).astype('uint8')))
            canvas.alpha_composite(part,tuple(low))
    x=(j%8)*400;y=(j//8)*480
    sheet.paste(canvas,(x,y),canvas);draw.text((x+16,y+452),f"{f['clip']}  {f['phase']:.0%}",fill='#142d3c')
sheet.save(OUT/'source-contact.png')
print(OUT/'source-contact.png')
