import bpy,json,sys,os
import numpy as np
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
ASSETS=Path(os.environ.get('TSB_CHARACTER_ASSET_DIR',ROOT/'assets/characters'))
OUT=Path(os.environ.get('TSB_FACE_REVIEW_DIR',ROOT/'artifacts/cheek-isolation'))
def asset(name):
    path=ASSETS/name
    return path if path.exists() else ROOT/'assets/characters'/name
for slug in (sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else ['chloe','renua']):
    bpy.ops.wm.open_mainfile(filepath=str(ASSETS/f'{slug}.blend'))
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig'];rig.data.pose_position='REST'
    face=json.loads(asset(f'{slug}-rig.json').read_text())['face']
    for key in body.data.shape_keys.key_blocks:key.value=0
    scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=12;scene.render.threads_mode='FIXED';scene.render.threads=4
    scene.render.resolution_x=scene.render.resolution_y=720;scene.render.resolution_percentage=100;scene.view_settings.view_transform='AgX'
    for obj in list(bpy.data.objects):
        if obj.type in ['CAMERA','LIGHT']:bpy.data.objects.remove(obj,do_unlink=True)
        elif obj.type=='MESH':obj.hide_render=('_RefinedViewSleeve.' in obj.name) or (slug=='renua' and '_RefinedSleeve.' in obj.name)
    if scene.world is None:scene.world=bpy.data.worlds.new('Face lighting')
    scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[1].default_value=.7
    center=Vector((face['center_x'],-face['front'],(face['eye_y']+face['mouth_y'])/2))
    for d,power in [((1,-2,2),100),((-1,-1,1),70)]:
        bpy.ops.object.light_add(type='AREA',location=center+Vector(d));obj=bpy.context.object;obj.data.energy=power;obj.data.size=2;obj.rotation_euler=(center-obj.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=center+Vector((0,-3,0)));cam=bpy.context.object;cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=.85;scene.camera=cam
    mat=body.data.materials[0];bsdf=mat.node_tree.nodes['Principled BSDF'];tex=mat.node_tree.nodes.new('ShaderNodeTexImage')
    mat.node_tree.links.new(tex.outputs['Color'],bsdf.inputs['Base Color'])
    original_normals=np.array([n.vector[:] for n in body.data.corner_normals])
    loops=np.array([l.vertex_index for l in body.data.loops])
    # Bake the evaluated positions for this diagnostic. Blender otherwise adds
    # its own shape-key normal transport on top of the runtime's explicit target.
    basis=np.array([v.co[:] for v in body.data.vertices])
    deltas={k.name:np.array([v.co[:] for v in k.data])-basis for k in body.data.shape_keys.key_blocks}
    body.shape_key_clear()
    phases=[('open',0,f'{slug}-meshy-basecolor.png'),('half',.52,f'{slug}-blink-half.png'),('closed',1,f'{slug}-blink-closed.png')]
    if os.environ.get('TSB_FACE_REVIEW_CLOSED_ONLY')=='1':phases=phases[-1:]
    for phase,closure,file in phases:
        tex.image=bpy.data.images.load(str(asset(file)),check_existing=False)
        normals=original_normals.copy()
        positions=basis.copy()
        for side in ['L','R']:
            positions+=deltas[f'eyeBlink_{side}']*closure+deltas[f'browDown_{side}']*closure*.75
            target=body.data.attributes.get(f'blink_normal_{side}')
            weight=body.data.attributes.get(f'blink_weight_{side}')
            if target and weight:
                n=np.array([v.vector[:] for v in target.data])[loops]
                w=np.array([v.value for v in weight.data])[loops]
                normals+=(n-original_normals)*w[:,None]*closure
        body.data.vertices.foreach_set('co',positions.astype(np.float32).ravel())
        body.data.update()
        body.data.normals_split_custom_set(normals.tolist())
        scene.render.filepath=str(OUT/f'{slug}-natural-{phase}.png');bpy.ops.render.render(write_still=True)
    if slug=='renua':
        body.data.vertices.foreach_set('co',basis.astype(np.float32).ravel())
        body.data.update();body.data.normals_split_custom_set(original_normals.tolist())
        tex.image=bpy.data.images.load(str(ASSETS/'renua-meshy-basecolor.png'),check_existing=False)
        target=Vector((0,0,1.0));cam.data.ortho_scale=2.30;cam.location=target+Vector((0,-4,0));cam.rotation_euler=(target-cam.location).to_track_quat('-Z','Y').to_euler()
        scene.render.filepath=str(OUT/'renua-restored-coat.png');bpy.ops.render.render(write_still=True)
