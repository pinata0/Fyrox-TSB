"""Render reproducible full-body/front/rear attack checkpoints from the saved blend."""
import bpy, sys, math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
label=sys.argv[sys.argv.index('--')+1] if '--' in sys.argv else 'after'
OUT=ROOT/'artifacts/vivi-skills'/label;OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/vivi.blend'))
scene=bpy.context.scene
for img in bpy.data.images:
    if img.source=='FILE' and not img.packed_file and not Path(bpy.path.abspath(img.filepath)).is_file():
        if 'refined' in img.name.lower() or 'arm' in img.name.lower():
            img.filepath=str(ROOT/'assets/characters/vivi-refined-arms.png');img.reload()
        print('REVIEW_IMAGE',img.name,img.filepath,flush=True)
for obj in bpy.context.scene.objects:
    if 'RefinedViewSleeve' in obj.name:obj.hide_render=True
scene.world=bpy.data.worlds.new('ReviewWorld');scene.world.color=(.28,.28,.28)
for offset,power in [((2,-4,5),450),((-3,-1,3),250),((0,3,4),400)]:
    bpy.ops.object.light_add(type='AREA',location=offset);light=bpy.context.object
    light.data.energy=power;light.data.size=4
    light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add();camera=bpy.context.object;scene.camera=camera
scene.render.engine='CYCLES';scene.cycles.samples=12;scene.cycles.use_denoising=True
scene.render.resolution_x=scene.render.resolution_y=640;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.film_transparent=False
camera.data.type='ORTHO'
for view,frame,position,target,scale in [
 ('idle-up',0,(0,-5,1.1),(0,0,1.1),2.8),
 ('idle-down',20,(0,-5,1.1),(0,0,1.1),2.8),
 ('skill-shoulder',630,(0,-5,1.1),(0,0,1.1),2.8),
 ('skill-raised',645,(0,-5,1.1),(0,0,1.1),2.8),
 ('ultimate',693,(2,-5,1.1),(0,0,1.1),2.8),
 ('rear-skill',645,(0,5,1.1),(0,0,1.1),2.8)
]:
 camera.location=position;camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.ortho_scale=scale
 scene.frame_set(frame)
 body=bpy.data.objects['vivi_SkinnedMesh']
 for key in body.data.shape_keys.key_blocks:
  key.value=0.0
  if False:
   if 'browUp_' in key.name:key.value=.95 if key.name.endswith('_L') else .72
   elif 'eyeWide_' in key.name:key.value=.90
   elif 'mouthFrown_' in key.name:key.value=.70
   elif key.name=='jawOpen':key.value=.60
 scene.render.filepath=str(OUT/f'{view}.png');bpy.ops.render.render(write_still=True)
print('VIVI_SKILLS_RENDER_COMPLETE',flush=True)



