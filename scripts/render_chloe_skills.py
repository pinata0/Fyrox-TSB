import bpy,sys
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import resolve_character_textures
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/chloe.blend'))
scene=bpy.context.scene;resolve_character_textures([o for o in scene.objects if o.type=='MESH'])
for o in list(scene.objects):
 if o.type in ['LIGHT','CAMERA']:bpy.data.objects.remove(o,do_unlink=True);continue
 if '_RefinedViewSleeve.' in o.name:o.hide_render=True
scene.render.engine='CYCLES';scene.cycles.samples=12;scene.render.resolution_x=600;scene.render.resolution_y=720;scene.render.resolution_percentage=100;scene.view_settings.view_transform='Standard'
scene.world=bpy.data.worlds.new('Chloe review');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.18,.22,.28,1)
for loc,power in [((3,-4,6),500),((-3,-3,3),300),((0,4,5),400)]:
 bpy.ops.object.light_add(type='AREA',location=loc);o=bpy.context.object;o.data.energy=power;o.data.size=5;o.rotation_euler=(Vector((0,0,1.6))-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(3,-7,3.2));scene.camera=bpy.context.object;scene.camera.data.type='ORTHO';scene.camera.data.ortho_scale=4;scene.camera.rotation_euler=(Vector((0,.2,1.6))-scene.camera.location).to_track_quat('-Z','Y').to_euler()
for name,frame in [('idle',0),('move',71),('mount',599),('ride',634),('ride-move',669),('attack',539),('roar',740)]:
 scene.frame_set(frame);scene.render.filepath=str(ROOT/f'artifacts/chloe-skills/{name}.png');bpy.ops.render.render(write_still=True)
