"""Archive the complete GPU run and make face-focused comparison sheets."""
import json,re,shutil,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'tools/python-libs'))
from PIL import Image,ImageDraw,ImageFont
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'artifacts/cheek-eyes'
log=(OUT/'gpu-final.log').read_text(encoding='utf8',errors='replace')
assert 'CHEEK_REVIEW_OK: 15 heroes' in log
frames={}
for line in log.splitlines():
    match=re.search(r'CHEEK_FRAME_OK: (\w+) phase=(\d).*centers=(.*)',line)
    if match:
        nums=list(map(float,re.findall(r'[\d.]+',match[3])))
        frames[(match[1],int(match[2]))]=((nums[0]+nums[2])/2,(nums[1]+nums[3])/2,abs(nums[2]-nums[0]))
assert len(frames)==105
profiles=json.loads((ROOT/'assets/characters/cheeks/eyes.json').read_text())
names={p['slug']:p['name'] for p in json.loads((ROOT/'assets/references/manifest.json').read_text(encoding='utf8'))}
font=ImageFont.truetype('C:/Windows/Fonts/malgun.ttf',19)
(OUT/'screenshots').mkdir(exist_ok=True)
def face_crop(slug,id,phase,size=(300,205)):
    source=Image.open(OUT/f'screenshots/preview-cheeks-{id:02}-{slug}-{phase}-menu.png')
    x,y,span=frames[(slug,phase)];scale=span/122
    return source.crop((int(x-150*scale),int(y-115*scale),int(x+150*scale),int(y+90*scale))).resize(size,Image.Resampling.LANCZOS)
for page in range(3):
    canvas=Image.new('RGB',(1500,1200),'#eaf0f5');draw=ImageDraw.Draw(canvas)
    for row,p in enumerate(profiles[page*5:page*5+5]):
        slug=p['slug'];id=page*5+row
        for phase in range(7):
            path=ROOT/f'artifacts/preview-cheeks-{id:02}-{slug}-{phase}-menu.png'
            shutil.copy2(path,OUT/'screenshots'/path.name)
        for col,(phase,title) in enumerate([(0,'기본'),(2,'왼쪽 당김'),(4,'놓은 뒤'),(5,'오른쪽 당김·진입'),(6,'복귀')]):
            im=face_crop(slug,id,phase)
            canvas.paste(im,(col*300,row*240+34));draw.text((col*300+9,row*240+7),names[slug]+' / '+title,font=font,fill='#1b2c3b')
    canvas.save(OUT/f'gpu-comparison-{page}.png')
canvas=Image.new('RGB',(1110,298),'#eaf0f5');draw=ImageDraw.Draw(canvas)
font=ImageFont.truetype('C:/Windows/Fonts/malgunbd.ttf',26)
for col,(phase,title) in enumerate([(0,'기본'),(2,'당기는 중'),(4,'놓은 뒤')]):
    canvas.paste(face_crop('vivi',0,phase,(370,253)),(col*370,45))
    draw.text((col*370+14,6),title,font=font,fill='#1b2c3b')
canvas.save(OUT/'vivi-final-comparison.png')
print('CHEEK_EYE_SHEETS_OK: 105 archived frames, 3 contact sheets')
