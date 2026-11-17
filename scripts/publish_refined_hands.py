"""Apply independently checked refined models, preserving the previous revision."""
import bpy,json,hashlib,shutil,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
OUT=ROOT/'artifacts/hand-mesh-polish';STAGE=OUT/'characters';DEST=ROOT/'assets/characters'
mesh_report=json.loads((OUT/'mesh-verification.json').read_text())
asset_report=json.loads((ROOT/'artifacts/meshy-import/verification.json').read_text())
assert mesh_report['passed'] and asset_report['passed']
assert len(mesh_report['characters'])==len(asset_report['characters'])==15
shutil.copy2(ROOT/'artifacts/meshy-import/verification.json',OUT/'asset-verification.json')
records=json.loads((DEST/'manifest.json').read_text(encoding='utf-8'))
for row in records:
    slug=row['slug'];shutil.copy2(STAGE/f'{slug}-refined-arms.png',DEST/f'{slug}-refined-arms.png')
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    for image in bpy.data.images:
        filename=Path(image.filepath.replace('\\','/')).name
        if (DEST/filename).is_file():image.filepath='//'+filename
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(DEST/f'{slug}.blend'),relative_remap=False)
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and any(m.type=='ARMATURE' and m.object==rig for m in o.modifiers)]
    export_fbx(body,rig,DEST/f'{slug}.fbx',extras)
    for suffix in ['-hands.json','-rig.json','.morphs.json']:shutil.copy2(STAGE/f'{slug}{suffix}',DEST/f'{slug}{suffix}')
    details=json.loads((DEST/f'{slug}-rig.json').read_text())
    row['triangles']=details['triangles'];row['bones']=details['bones'];row['refined_hands']=details['refined_hands']
    row['refined_arms_texture']=f'assets/characters/{slug}-refined-arms.png'
    for key in ['blend','model','hands','refined_arms_texture']:
        row['sha256'][key]=hashlib.sha256((ROOT/row[key]).read_bytes()).hexdigest()
    print('REFINED_HANDS_APPLIED',slug,flush=True)
(DEST/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
