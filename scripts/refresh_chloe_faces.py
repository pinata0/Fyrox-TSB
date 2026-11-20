"""Refresh geometric expressions without rebaking the verified skeletal timeline."""
import bpy,sys,json,hashlib
from pathlib import Path
P=Path(__file__).resolve().parents[1];sys.path.insert(0,str(P/'scripts'))
from chloe_face_meshes import build_faces
from export_fyrox_fbx import export_fbx
bpy.ops.wm.open_mainfile(filepath=str(P/'assets/characters/chloe.blend'))
bpy.context.scene.frame_set(0)
for name in ['ChloeScissorEyes','ChloeMoveEyes','SebastianClosedEyes']:
    bpy.data.objects.remove(bpy.data.objects[name],do_unlink=True)
rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['chloe_SkinnedMesh']
build_faces(rig,body,bpy.data.objects['Sebastian'])
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(P/'assets/characters/chloe.blend'))
export_fbx(body,rig,P/'assets/characters/chloe.fbx',bake_step=.5)
path=P/'assets/characters/manifest.json';records=json.loads(path.read_text(encoding='utf8'))
record=next(r for r in records if r['slug']=='chloe')
for key in ['blend','model']:record['sha256'][key]=hashlib.sha256((P/record[key]).read_bytes()).hexdigest()
path.write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print('CHLOE_FACE_REFRESH_OK',flush=True)
