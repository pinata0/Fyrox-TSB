"""Model the generated medical concept as beveled meshes with independent lid/drum."""
import bpy,json,math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/effects';REVIEW=ROOT/'artifacts/hilde-skills';REVIEW.mkdir(parents=True,exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
palette={'ivory':(.88,.79,.62,1),'dark':(.12,.105,.11,1),'lime':(.68,.85,.16,1),'gold':(.59,.37,.14,1),'inside':(.04,.035,.03,1)}
mats={}
for name,c in palette.items():
 m=bpy.data.materials.new(name);m.diffuse_color=c;m.use_nodes=True;m.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=c;m.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.65;mats[name]=m
parts=[]
def convert(p):return (p[0],-p[2],p[1])
def add(name,group,p,size,mat,kind='box',axis='z'):
 if kind=='box':bpy.ops.mesh.primitive_cube_add(size=1,location=convert(p))
 else:bpy.ops.mesh.primitive_cylinder_add(vertices=24,radius=.5,depth=1,location=convert(p))
 o=bpy.context.object;o.name=name;o.scale=convert((size[0],size[1],-size[2])) if kind=='box' else size
 if kind!='box':o.rotation_euler=(math.pi/2,0,0) if axis=='z' else (0,math.pi/2,0) if axis=='x' else (0,0,0)
 bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
 mod=o.modifiers.new('Soft manufactured edges','BEVEL');mod.width=.012 if kind=='box' else .004;mod.segments=2;bpy.ops.object.modifier_apply(modifier=mod.name)
 o.data.materials.append(mats[mat]);o['part_group']=group;parts.append(o)
 return o
def box(n,g,p,s,m):return add(n,g,p,s,m)
def cyl(n,g,p,r,length,m,axis='z'):return add(n,g,p,(r*2,r*2,length),m,'cylinder',axis)
def muzzle_ring():
 verts=[];faces=[]
 for z,r in [(.61,.116),(.69,.116),(.69,.082),(.635,.082)]:
  for i in range(32):a=i*math.tau/32;verts.append(convert((math.cos(a)*r,.18+math.sin(a)*r,z)))
 for row in range(3):
  for i in range(32):j=(i+1)%32;faces.append((row*32+i,row*32+j,(row+1)*32+j,(row+1)*32+i))
 mesh=bpy.data.meshes.new('Hollow muzzle');mesh.from_pydata(verts,[],faces);mesh.update();o=bpy.data.objects.new('Hollow dark muzzle ring',mesh);bpy.context.collection.objects.link(o);mesh.materials.append(mats['dark']);o['part_group']='launcher';parts.append(o)
g='case'
box('Ivory case back',g,(0,-.235,-.065),(.53,.35,.065),'ivory')
box('Dark fitted interior',g,(0,-.235,-.027),(.47,.29,.025),'inside')
for x in [-.25,.25]:box('Protective case side',g,(x,-.235,.01),(.045,.35,.17),'dark')
for y in [-.40,-.07]:box('Ivory case rim',g,(0,y,.01),(.50,.035,.17),'ivory')
for x in [-.11,.11]:box('Handle post',g,(x,-.035,0),(.045,.10,.06),'dark')
box('Carrying handle',g,(0,.015,0),(.26,.045,.06),'dark')
for x in [-.15,-.05,.05,.15]:
 cyl('Medicine vial',g,(x,-.25,.012),.034,.11,'lime',axis='y')
 box('Vial cap',g,(x,-.19,.025),(.046,.025,.05),'gold')
g='lid'
box('Independent ivory opening lid',g,(0,-.235,.108),(.51,.33,.042),'ivory')
cyl('Round medical badge',g,(0,-.235,.135),.105,.014,'dark')
box('Lime medical cross vertical',g,(0,-.235,.147),(.043,.139,.018),'lime')
box('Lime medical cross horizontal',g,(0,-.235,.147),(.139,.043,.018),'lime')
for x in [-.20,.20]:
 for y in [-.365,-.105]:box('Dark rounded case corner',g,(x,y,.13),(.095,.074,.035),'dark')
for x in [-.15,.15]:box('Brass case latch',g,(x,-.08,.145),(.04,.063,.027),'gold')
g='launcher'
box('Ivory receiver',g,(0,.15,-.06),(.22,.22,.32),'ivory')
box('Trigger grip',g,(0,-.015,-.03),(.10,.23,.12),'dark')
box('Short shoulder stock',g,(0,.13,-.38),(.16,.14,.30),'ivory')
box('Soft stock pad',g,(0,.10,-.54),(.18,.26,.055),'dark')
cyl('Barrel sleeve',g,(0,.18,.46),.105,.34,'ivory')
muzzle_ring()
cyl('Deep recessed bore',g,(0,.18,.636),.082,.006,'inside')
box('Foregrip bracket',g,(0,.065,.28),(.13,.06,.16),'dark')
box('Support foregrip',g,(0,-.025,.28),(.09,.17,.09),'dark')
box('Upper bridge',g,(0,.335,.16),(.15,.045,.49),'ivory')
box('Top sight',g,(0,.38,.32),(.045,.065,.04),'dark')
for x in [-.118,.118]:
 box('Side medical emblem vertical',g,(x,.16,-.06),(.017,.11,.033),'lime')
 box('Side medical emblem horizontal',g,(x,.16,-.06),(.017,.033,.11),'lime')
g='drum'
cyl('Six capsule rotary drum',g,(0,.14,.19),.154,.24,'dark')
for i in range(6):
 a=i*math.tau/6;x=math.cos(a)*.132;y=.14+math.sin(a)*.132
 cyl('Lime medical capsule '+str(i),g,(x,y,.19),.042,.20,'lime')
 cyl('Capsule brass base '+str(i),g,(x,y,.075),.044,.03,'gold')
 cyl('Capsule ivory nose '+str(i),g,(x,y,.30),.043,.045,'ivory')
# Evaluate bevels and preserve flat normals in the game mesh export.
groups=[]
for group in ['case','lid','launcher','drum']:
 for mat,color in palette.items():
  verts=[];tris=[]
  for o in parts:
   if o['part_group']!=group or o.data.materials[0].name!=mat:continue
   o.data.calc_loop_triangles()
   for tri in o.data.loop_triangles:
    base=len(verts)
    for idx in tri.vertices:
     p=o.matrix_world@o.data.vertices[idx].co;n=o.matrix_world.to_3x3()@tri.normal
     verts.append([round(v,6) for v in [p.x,p.z,-p.y,n.x,n.z,-n.y]])
    tris.append([base,base+1,base+2])
  if verts:groups.append(dict(group=group,color=[round(c*255) for c in color[:3]],vertices=verts,triangles=tris))
(OUT/'hilde-medical-props.json').write_text(json.dumps(groups,separators=(',',':')))
# Arrange both models for a review render only; runtime meshes use the grip origin.
for o in parts:
 if o['part_group'] in ['case','lid']:o.location.x-=.70
 else:o.location.x+=.45
scene=bpy.context.scene;scene.world.color=(.16,.16,.16)
for pos,power in [((1,-3,5),450),((-3,-1,2),250),((0,4,3),450)]:
 bpy.ops.object.light_add(type='AREA',location=pos);o=bpy.context.object;o.data.energy=power;o.data.size=4;o.rotation_euler=(-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(2.4,-3.3,1.8));cam=bpy.context.object;cam.rotation_euler=(Vector((0,0,0))-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=2.3;scene.camera=cam
scene.render.engine='CYCLES';scene.cycles.samples=24;scene.cycles.use_denoising=True;scene.render.resolution_x=1200;scene.render.resolution_y=800;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.filepath=str(REVIEW/'medical-props-model.png');bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'hilde-medical-props.blend'));bpy.ops.render.render(write_still=True)
print('HILDE_PROPS_OK',sum(len(g['triangles']) for g in groups),flush=True)
