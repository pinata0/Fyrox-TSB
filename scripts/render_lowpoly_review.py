"""Render the staged rigs and their actual expression textures for inspection."""
import bpy
import sys
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];WORK=ROOT/'artifacts/lowpoly-import'

for slug in sys.argv[sys.argv.index('--')+1:]:
    for version in ['before','characters']:
        bpy.ops.wm.open_mainfile(filepath=str(WORK/version/f'{slug}.blend'))
        scene=bpy.context.scene;scene.frame_set(0)
        rig=bpy.data.objects['TSB_Rig'];rig.data.pose_position='POSE'
        for obj in list(scene.objects):
            if obj.type in {'CAMERA','LIGHT'}:bpy.data.objects.remove(obj,do_unlink=True)
            elif obj.type=='MESH':obj.hide_render='_RefinedViewSleeve.' in obj.name
        for image in bpy.data.images:
            path=WORK/version/Path(bpy.path.abspath(image.filepath)).name
            if path.is_file():image.filepath=str(path)
        scene.render.engine='CYCLES';scene.cycles.samples=8
        scene.render.resolution_x=600;scene.render.resolution_y=700;scene.render.resolution_percentage=100
        scene.view_settings.view_transform='Standard'
        if scene.world is None:scene.world=bpy.data.worlds.new('ReviewWorld')
        scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.28,.30,.34,1)
        scene.world.node_tree.nodes['Background'].inputs[1].default_value=.7
        center=Vector((0,0,1.0))
        for position,power in [((3,-4,5),400),((-3,-2,3),180),((0,3,4),250)]:
            bpy.ops.object.light_add(type='AREA',location=position);light=bpy.context.object
            light.data.energy=power;light.data.size=4;light.rotation_euler=(center-light.location).to_track_quat('-Z','Y').to_euler()
        bpy.ops.object.camera_add(location=(0,-6,1.05));camera=bpy.context.object
        camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler()
        camera.data.type='ORTHO';camera.data.ortho_scale=2.4;scene.camera=camera
        scene.render.filepath=str(WORK/f'{slug}-{version}.png');bpy.ops.render.render(write_still=True)
        if True:
            body=bpy.data.objects[slug+'_SkinnedMesh']
            for side in ['L','R']:body.data.shape_keys.key_blocks['eyeBlink_'+side].value=1
            material=body.data.materials[0]
            base=next(n for n in material.node_tree.nodes if n.type=='TEX_IMAGE' and n.image.name==material['meshy_base_texture'])
            base.image=bpy.data.images.load(str(WORK/version/f'{slug}-blink-closed.png'))
            camera.data.ortho_scale=1.05;camera.location.z=1.48
            camera.rotation_euler=(Vector((0,0,1.48))-camera.location).to_track_quat('-Z','Y').to_euler()
            scene.render.filepath=str(WORK/f'{slug}-{version}-blink.png');bpy.ops.render.render(write_still=True)
