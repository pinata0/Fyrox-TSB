import bpy,sys,json,hashlib,shutil
from pathlib import Path
root=Path('C:/Users/user/projects/Fyrox-TSB');sys.path.insert(0,str(root/'scripts'))
from export_fyrox_fbx import export_fbx
out=root/'artifacts/death-replay';backup=out/'before-fps-fix';backup.mkdir(exist_ok=True)
report=[]
for slug in ['naia','erpin','renua']:
 blend=root/f'assets/characters/{slug}.blend';fbx=blend.with_suffix('.fbx')
 for p in [blend,fbx]:
  if not (backup/p.name).exists():shutil.copy2(p,backup/p.name)
 bpy.ops.wm.open_mainfile(filepath=str(blend));scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig']
 old=scene.render.fps;scene.render.fps=30;scene.render.fps_base=1.0
 bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(blend))
 export_fbx(bpy.data.objects[f'{slug}_SkinnedMesh'],rig,fbx)
 report.append({'slug':slug,'old_fps':old,'fps':30,'fbx_sha256':hashlib.sha256(fbx.read_bytes()).hexdigest()})
 print('FPS_FIXED',slug,old,'->30',flush=True)
(out/'fps-fix.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
