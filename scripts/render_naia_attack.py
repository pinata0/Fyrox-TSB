"""Render reproducible full-body/front/rear attack checkpoints from the saved blend."""
import bpy, sys, math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
label=sys.argv[sys.argv.index('--')+1] if '--' in sys.argv else 'after'
OUT=ROOT/'artifacts/naia-attack'/label;OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/naia.blend'))
scene=bpy.context.scene
for img in bpy.data.images:
    if img.source=='FILE' and not img.packed_file and not Path(bpy.path.abspath(img.filepath)).is_file():
        if 'refined' in img.name.lower() or 'arm' in img.name.lower():
            img.filepath=str(ROOT/'assets/characters/naia-refined-arms.png');img.reload()
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
for view,position,target,scale in [('front',(0,-5,1.0),(0,0,1.0),2.1),('back',(0,5,1.0),(0,0,1.0),2.1),('side',(5,0,1.0),(0,0,1.0),2.1)]:
    camera.location=position;camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.ortho_scale=scale
    for phase in ([0] if label=='before' else [0,.3,.54,.64,1]):
        f=156+24*phase;scene.frame_set(int(f),subframe=f-int(f))
        scene.render.filepath=str(OUT/f'{view}-{phase:.2f}.png');bpy.ops.render.render(write_still=True)
print('NAIA_RENDER_COMPLETE',label,flush=True)
