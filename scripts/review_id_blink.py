"""Render Id's actual morphed mesh and production atlases, front and oblique."""
import bpy,sys
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/id-blink-fix'
folder=Path(sys.argv[sys.argv.index('--')+1]).resolve() if '--' in sys.argv else OUT/'candidate'
if '--output-dir' in sys.argv:
    OUT=Path(sys.argv[sys.argv.index('--output-dir')+1]).resolve();OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(folder/'id.blend'))
scene=bpy.context.scene;body=bpy.data.objects['id_SkinnedMesh']
for obj in list(scene.objects):
    if obj.type in {'CAMERA','LIGHT'}:bpy.data.objects.remove(obj,do_unlink=True)
    elif obj.type=='MESH':obj.hide_render=obj!=body
for modifier in body.modifiers:modifier.show_render=False
body.matrix_world.identity()
scene.render.engine='CYCLES';scene.cycles.samples=8
scene.render.resolution_x=600;scene.render.resolution_y=600;scene.render.resolution_percentage=100
scene.view_settings.view_transform='Standard'
scene.world=bpy.data.worlds.new('Id blink review');scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.16,.18,.21,1)
mat=bpy.data.materials.new('Id blink atlas review');mat.use_nodes=True
nodes=mat.node_tree.nodes;nodes.clear()
texture=nodes.new('ShaderNodeTexImage');texture.interpolation='Linear'
emission=nodes.new('ShaderNodeEmission');output=nodes.new('ShaderNodeOutputMaterial')
mat.node_tree.links.new(texture.outputs['Color'],emission.inputs['Color'])
mat.node_tree.links.new(emission.outputs[0],output.inputs['Surface'])
for i in range(len(body.data.materials)):body.data.materials[i]=mat
center=Vector((-.004,0,1.29))
bpy.ops.object.camera_add(location=center+Vector((0,-3,0)))
camera=bpy.context.object;scene.camera=camera;camera.data.type='ORTHO';camera.data.ortho_scale=.64
for view,offset in [('front',(0,-3,0)),('oblique',(.8,-3,.05))]:
    camera.location=center+Vector(offset)
    camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler()
    for tag,closure,suffix in [('open',0,'-meshy-basecolor'),('half',.52,'-blink-half'),('closed',1,'-blink-closed')]:
        path=(ROOT/'assets/characters' if closure==0 else folder)/f'id{suffix}.png'
        texture.image=bpy.data.images.load(str(path),check_existing=False)
        for key in body.data.shape_keys.key_blocks:
            key.value=closure if key.name.startswith('eyeBlink_') else .75*closure if key.name.startswith('browDown_') else 0
        scene.render.filepath=str(OUT/f'{folder.name}-{view}-{tag}.png')
        bpy.ops.render.render(write_still=True)
        print('ID_BLINK_RENDER',view,tag,flush=True)
