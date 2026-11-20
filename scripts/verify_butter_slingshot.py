"""Check the repaired fork clearance throughout the giant draw and render it."""
import bpy, json, math
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/butter-skills/slingshot-fix'
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/butter-weapon.blend'))
obj=bpy.data.objects['butter_weapon'];mesh=obj.data;keys=mesh.shape_keys.key_blocks
base=[v.co.copy() for v in keys['Basis'].data]
rigid={i for i,p in enumerate(base) if (keys['bandPull'].data[i].co-p).length<1e-7}
faces=[list(p.vertices) for p in mesh.polygons if all(i in rigid for i in p.vertices)]
tree=BVHTree.FromPolygons(base,faces)
clearances=[]
for i in range(101):
    t=i/100
    center=Vector((0,.32*t-.065,.265))
    nearest=tree.find_nearest(center)
    clearances.append(nearest[3]-.060)
print('BUTTER_FRAME_CLEARANCE',min(clearances),len(rigid),flush=True)
assert min(clearances)>.003, 'loaded boulder intersects rigid fork'
(OUT/'clearance.json').write_text(json.dumps({'draw_samples':101,'boulder_bounding_radius':.060,'minimum_frame_gap':min(clearances),'rigid_vertices':len(rigid)},indent=2))
scene=bpy.context.scene
scene.render.engine='CYCLES';scene.cycles.samples=24
scene.render.resolution_x=900;scene.render.resolution_y=800;scene.render.resolution_percentage=100
scene.world=bpy.data.worlds.new('Butter QA world');scene.world.color=(.25,.25,.25)
scene.view_settings.view_transform='Standard'
bpy.ops.object.camera_add(location=(.65,1.6,.85));cam=bpy.context.object
cam.rotation_euler=(Vector((0,.14,.20))-cam.location).to_track_quat('-Z','Y').to_euler()
cam.data.type='ORTHO';cam.data.ortho_scale=.72;scene.camera=cam
bpy.ops.object.light_add(type='AREA',location=(1,2,2));bpy.context.object.data.energy=150;bpy.context.object.data.shape='DISK';bpy.context.object.data.size=3
mat=bpy.data.materials.new('Boulder');mat.diffuse_color=(.42,.36,.28,1)
bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2,radius=.060);rock=bpy.context.object;rock.data.materials.append(mat)
for label,pull,lift in [('rest',0,0),('giant-start',0,1),('giant-middle',.5,1),('giant-drawn',1,1)]:
    keys['bandPull'].value=pull if not lift else 0;keys['giantBandLift'].value=lift;keys['giantBandPull'].value=pull if lift else 0
    rock.hide_render=not lift;rock.location=(0,.32*pull-.065,.265)
    scene.render.filepath=str(OUT/f'{label}.png');bpy.ops.render.render(write_still=True)
print('BUTTER_SLINGSHOT_VERIFIED',flush=True)
