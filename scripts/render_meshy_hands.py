import bpy,json,sys
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/meshy-hand-fix';OUT.mkdir(parents=True,exist_ok=True)
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else ['vivi','erpin','shion','naia','butter']
for slug in selected:
    bpy.ops.wm.open_mainfile(filepath=str(ROOT/f'assets/characters/{slug}.blend'))
    mesh=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig'];scene=bpy.context.scene;scene.frame_set(0)
    for o in list(bpy.data.objects):
        if o.type in {'CAMERA','LIGHT'}:bpy.data.objects.remove(o,do_unlink=True)
    scene.render.engine='CYCLES';scene.cycles.samples=12;scene.render.resolution_x=600;scene.render.resolution_y=600;scene.render.resolution_percentage=100
    scene.view_settings.view_transform='Standard'
    if scene.world is None:scene.world=bpy.data.worlds.new('Hand review world')
    scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[1].default_value=.8
    wrist=rig.data.bones['Hand.L'].head_local.copy();elbow=rig.data.bones['Forearm.L'].head_local
    forward=(wrist-elbow).normalized();center=wrist+forward*.05
    for loc,energy in [((1,-3,3),300),((-2,-1,1),180)]:
        bpy.ops.object.light_add(type='AREA',location=center+Vector(loc));o=bpy.context.object;o.data.energy=energy;o.data.size=2
        o.rotation_euler=(center-o.location).to_track_quat('-Z','Y').to_euler()
    for label,direction in [('front',(0,-1,.2)),('palm',(0,1,-.2))]:
        bpy.ops.object.camera_add(location=center+Vector(direction));cam=bpy.context.object;cam.data.type='ORTHO';cam.data.ortho_scale=.34
        cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler();scene.camera=cam
        scene.render.filepath=str(OUT/f'{slug}-hand-before-{label}.png');bpy.ops.render.render(write_still=True)
        bpy.data.objects.remove(cam,do_unlink=True)
