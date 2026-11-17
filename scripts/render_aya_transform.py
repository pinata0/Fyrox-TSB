"""Render the editable weapon states at a common scale for visual QA."""
import bpy
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/aya-transform/model-review';OUT.mkdir(exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/aya-weapon.blend'))
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=12
scene.render.resolution_x=480;scene.render.resolution_y=640;scene.render.resolution_percentage=100
scene.view_settings.view_transform='Standard'
scene.world=bpy.data.worlds.new('Aya review');scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.13,.17,.22,1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value=.5
center=Vector((0,0,1.12))
for loc,power in [((3,-4,5),350),((-3,-2,3),200),((0,3,4),300)]:
 bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=power;light.data.size=4
 light.rotation_euler=(center-light.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=center+Vector((.5,-5,.55)));camera=bpy.context.object
camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler()
camera.data.type='ORTHO';camera.data.ortho_scale=2.65;scene.camera=camera
for name,frame in [('open',0),('folding',3),('sword',18),('long-blade',120),('glaive',150)]:
 scene.frame_set(frame);scene.render.filepath=str(OUT/f'{name}.png');bpy.ops.render.render(write_still=True)
print('AYA_WEAPON_RENDER_OK',flush=True)
