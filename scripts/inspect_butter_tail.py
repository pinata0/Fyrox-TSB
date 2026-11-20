import bpy,json,sys
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from import_tripo_butter import components,bounds
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/butter.blend'));scene=bpy.context.scene;scene.frame_set(0)
o=bpy.data.objects['butter_SkinnedMesh']
records=[]
for ids in components(o.data):
 bb=bounds(o,ids)
 if len(ids)>15:records.append({'vertices':len(ids),'bounds':bb,'indices':ids[:4]})
(ROOT/'artifacts/butter-skills/components.json').write_text(json.dumps(records,indent=2))
bpy.ops.object.camera_add(location=(2.8,5,2.2));camera=bpy.context.object;scene.camera=camera;camera.rotation_euler=(Vector((0,0,1))-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=2.3
for pos in [(2,4,5),(-3,2,3)]:
 bpy.ops.object.light_add(type='AREA',location=pos);light=bpy.context.object;light.data.energy=400;light.data.shape='DISK';light.data.size=4;light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
scene.render.resolution_x=512;scene.render.resolution_y=512;scene.render.resolution_percentage=100;scene.render.image_settings.file_format='PNG';scene.render.filepath=str(ROOT/'artifacts/butter-skills/back.png');scene.render.engine='CYCLES';scene.cycles.samples=8
bpy.ops.render.render(write_still=True)
