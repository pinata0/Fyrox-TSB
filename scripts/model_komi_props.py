"""Editable quilt and opened cat-food tin; export the same meshes used in game."""
import bpy, json, math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/effects';WORK=ROOT/'artifacts/komi-skills'
WORK.mkdir(parents=True,exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
palette={'quilt':(.57,.23,.40,1),'pink':(.95,.56,.69,1),'cream':(.98,.88,.80,1),'purple':(.22,.13,.30,1),'metal':(.65,.70,.74,1),'food':(.36,.16,.065,1),'gold':(.87,.52,.18,1)}
mats={};parts=[]
for n,c in palette.items():
 m=bpy.data.materials.new(n);m.diffuse_color=c;m.use_nodes=True;m.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=c;m.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.35 if n=='metal' else .8;mats[n]=m
def coord(p):return (p[0],-p[2],p[1])
def finish(o,n,g,m):
 o.name=n;o.data.materials.append(mats[m]);o['part_group']=g;parts.append(o);return o
def ellipsoid(n,g,p,s,m):
 bpy.ops.mesh.primitive_uv_sphere_add(segments=24,ring_count=12,location=coord(p));o=bpy.context.object;o.scale=(s[0],s[2],s[1]);bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
 for f in o.data.polygons:f.use_smooth=True
 return finish(o,n,g,m)
def torus(n,g,p,r,t,m):
 bpy.ops.mesh.primitive_torus_add(major_segments=40,minor_segments=8,location=coord(p),major_radius=r,minor_radius=t);return finish(bpy.context.object,n,g,m)
def mesh(n,g,verts,faces,m):
 d=bpy.data.meshes.new(n);d.from_pydata([coord(v) for v in verts],[],faces);d.update();o=bpy.data.objects.new(n,d);bpy.context.collection.objects.link(o);return finish(o,n,g,m)
def tube(n,g,points,r,m):
 c=bpy.data.curves.new(n,'CURVE');c.dimensions='3D';c.bevel_depth=r;c.bevel_resolution=2;p=c.splines.new('POLY');p.points.add(len(points)-1)
 for point,co in zip(p.points,points):point.co=(*coord(co),1)
 o=bpy.data.objects.new(n,c);bpy.context.collection.objects.link(o);bpy.context.view_layer.objects.active=o;o.select_set(True);bpy.ops.object.convert(target='MESH');o=bpy.context.object;o.select_set(False);return finish(o,n,g,m)
def height(x,z):
 return .13+.43*max(0,1-(x/.69)**2)**.6+.016*math.cos(z*21)*math.sin(x*16)**2
# Rounded duvet with actual thickness and a folded cream upper hem.
nx=36;nz=36;verts=[];faces=[]
for j in range(nz+1):
 z=-.86+1.20*j/nz
 for i in range(nx+1):
  x=-.69+1.38*i/nx;verts.append((x,height(x,z),z))
for j in range(nz):
 for i in range(nx):a=j*(nx+1)+i;faces.append((a,a+nx+1,a+nx+2,a+1))
o=mesh('Soft quilt draped over resting body','blanket',verts,faces,'quilt');mod=o.modifiers.new('Padded fabric thickness','SOLIDIFY');mod.thickness=.035;bpy.context.view_layer.objects.active=o;bpy.ops.object.modifier_apply(modifier=mod.name)
for p in o.data.polygons:p.use_smooth=True
for z in [-.86,.34]:tube('Cream piped duvet edge','blanket',[(x,height(x,z)+.01,z) for x in [-.69+i*1.38/48 for i in range(49)]],.026,'cream')
for x in [-.69,.69]:tube('Soft pink side piping','blanket',[(x,height(x,z),z) for z in [-.86+i*1.2/40 for i in range(41)]],.027,'pink')
for j in range(5):
 z=-.72+j*.215
 tube('Quilt stitched channel','blanket',[(x,height(x,z)+.014,z) for x in [-.65+i*1.3/32 for i in range(33)]],.004,'pink')
for x in [-.43,-.21,0,.21,.43]:
 for z in [-.64,-.21,.16]:
  ellipsoid('Cream polka dot','blanket',(x,height(x,z)+.018,z),(.032,.007,.032),'cream')
ellipsoid('Soft sleeping pillow','blanket',(0,.12,.72),(.54,.12,.30),'pink')
for x in [-.50,.50]:
 for j in range(9):ellipsoid('Pillow ruffle','blanket',(x,.12,.49+j*.056),(.065,.036,.046),'cream')
# The can's origin is its base. Its wall is hollow, with a visible rolled rim.
verts=[];faces=[];rings=[(.21,0),(.21,.43),(.186,.43),(.186,.03)]
for r,y in rings:
 for i in range(48):a=i*math.tau/48;verts.append((r*math.cos(a),y,r*math.sin(a)))
for row in range(3):
 for i in range(48):j=(i+1)%48;faces.append((row*48+i,row*48+j,(row+1)*48+j,(row+1)*48+i))
mesh('Hollow food can','can',verts,faces,'purple')
torus('Rolled silver top rim','can',(0,.43,0),.203,.017,'metal');torus('Rolled silver bottom rim','can',(0,.016,0),.207,.015,'metal')
for y in [.08,.36]:torus('Pink label border','can',(0,y,0),.212,.014,'pink')
ellipsoid('Cream label badge','can',(0,.22,.211),(.13,.12,.008),'cream')
ellipsoid('Paw pad badge','can',(0,.20,.222),(.056,.042,.008),'quilt')
for x,y in [(-.068,.255),(-.024,.28),(.024,.28),(.068,.255)]:ellipsoid('Paw toe badge','can',(x,y,.222),(.021,.025,.009),'quilt')
ellipsoid('Food surface','can',(0,.375,0),(.181,.025,.181),'food')
for i in range(24):
 a=i*2.39996;r=.15*math.sqrt(i/24);ellipsoid('Individual kibble','can',(r*math.cos(a),.40+.008*math.sin(i),r*math.sin(a)),(.027,.019,.021),'gold' if i%3==0 else 'food')
# A separately transformable metal lid, peeled back from the rim.
ellipsoid('Independent peeled can lid','lid',(0,0,0),(.195,.012,.195),'metal')
torus('Pull ring','lid',(0,.017,.045),.048,.009,'purple')
groups=[]
for group in ['blanket','can','lid']:
 for mat,c in palette.items():
  vertices=[];triangles=[]
  for o in parts:
   if o['part_group']!=group or o.data.materials[0].name!=mat:continue
   o.data.calc_loop_triangles()
   for tri in o.data.loop_triangles:
    base=len(vertices)
    for idx in tri.vertices:
     p=o.matrix_world@o.data.vertices[idx].co;n=o.matrix_world.to_3x3()@o.data.vertices[idx].normal
     vertices.append([round(v,6) for v in (p.x,p.z,-p.y,n.x,n.z,-n.y)])
    triangles.append([base,base+1,base+2])
  if vertices:groups.append(dict(group=group,color=[round(v*255) for v in c[:3]],vertices=vertices,triangles=triangles))
(OUT/'komi-props.json').write_text(json.dumps(groups,separators=(',',':')))
# Save independent editable assets before arranging a review shot.
for group,name in [('blanket','komi-blanket'),('can','komi-food-can')]:
 selected=[o for o in parts if o['part_group']==group or group=='can' and o['part_group']=='lid']
 bpy.ops.object.select_all(action='DESELECT')
 for o in selected:o.select_set(True)
 bpy.ops.export_scene.gltf(filepath=str(OUT/(name+'.glb')),use_selection=True,export_format='GLB')
for o in parts:
 if o['part_group']=='blanket':o.location.x-=.6
 else:
  if o['part_group']=='lid':o.rotation_euler.x=-1.8;o.location.y=.20;o.location.z=.44
  o.location*=1.5;o.location.x+=.68;o.scale*=1.5
scene=bpy.context.scene;scene.world.color=(.16,.16,.16)
for pos,power in [((1,-3,5),450),((-3,-1,3),350),((0,4,4),500)]:
 bpy.ops.object.light_add(type='AREA',location=pos);o=bpy.context.object;o.data.energy=power;o.data.size=4;o.rotation_euler=(-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(2.2,-3.5,3.3));cam=bpy.context.object;cam.rotation_euler=(Vector((0,0,.25))-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=3.3;scene.camera=cam
scene.render.engine='CYCLES';scene.cycles.samples=24;scene.render.resolution_x=1200;scene.render.resolution_y=800;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.filepath=str(WORK/'props-model.png');bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'komi-props.blend'));bpy.ops.render.render(write_still=True)
print('KOMI_PROPS_OK',sum(len(g['triangles']) for g in groups),flush=True)
