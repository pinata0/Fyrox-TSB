"""Model the approved source-inspired concept as game meshes and editable Blender/GLB."""
import bpy,json,math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/effects';REVIEW=ROOT/'artifacts/mayo-skills';REVIEW.mkdir(parents=True,exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
palette={'sage':(.39,.43,.29,1),'ivory':(.79,.72,.60,1),'brown':(.23,.13,.12,1),'steel':(.40,.32,.30,1),'dark':(.055,.035,.035,1),'toxin':(.45,.68,.075,1)}
mats={}
for n,c in palette.items():
 m=bpy.data.materials.new(n);m.diffuse_color=c;m.use_nodes=True;m.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=c;m.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.65;mats[n]=m
parts=[]
def xyz(p):return (p[0],-p[2],p[1])
def finish(o,n,g,m):
 o.name=n;o['group']=g;o.data.materials.append(mats[m]);parts.append(o)
 return o
def box(n,g,p,s,m):
 bpy.ops.mesh.primitive_cube_add(size=1,location=xyz(p));o=bpy.context.object;o.scale=(s[0],s[2],s[1]);bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
 mod=o.modifiers.new('Rounded silhouette','BEVEL');mod.width=.012;mod.segments=2;bpy.ops.object.modifier_apply(modifier=mod.name)
 return finish(o,n,g,m)
def cyl(n,g,p,r,length,m,axis='z'):
 bpy.ops.mesh.primitive_cylinder_add(vertices=20,radius=r,depth=length,location=xyz(p));o=bpy.context.object
 o.rotation_euler=(math.pi/2,0,0) if axis=='z' else (0,math.pi/2,0) if axis=='x' else (0,0,0)
 return finish(o,n,g,m)
def bar(n,g,a,b,r,m):
 d=Vector(xyz(b))-Vector(xyz(a));o=cyl(n,g,tuple((a[i]+b[i])/2 for i in range(3)),r,d.length,m,axis='y');o.rotation_euler=d.to_track_quat('Z','Y').to_euler();return o
def tube(n,g,p,r,inner,length,m):
 verts=[];faces=[]
 for z,rad in [(p[2]-length/2,r),(p[2]+length/2,r),(p[2]+length/2,inner),(p[2]-length/2,inner)]:
  for i in range(24):a=i*math.tau/24;verts.append(xyz((p[0]+math.cos(a)*rad,p[1]+math.sin(a)*rad,z)))
 for j in range(4):
  for i in range(24):k=(i+1)%24;faces.append((j*24+i,j*24+k,((j+1)%4)*24+k,((j+1)%4)*24+i))
 mesh=bpy.data.meshes.new(n);mesh.from_pydata(verts,[],faces);mesh.update();o=bpy.data.objects.new(n,mesh);bpy.context.collection.objects.link(o);finish(o,n,g,m)
def common(g,heavy):
 box('Sage receiver',g,(0,.16,.04),(.20 if heavy else .14,.20,.29),'sage')
 box('Brown rear pistol grip',g,(0,0,0),(.095,.20,.105),'brown')
 for y in [-.065,-.025,.015]:box('Grip rib',g,(0,y,.005),(.101,.012,.11),'steel')
 for x in [-.045,.045]:
  bar('Open trigger guard',g,(x,.11,.10),(x,-.07,.12),.013,'ivory');bar('Trigger guard lower',g,(x,-.07,.12),(x,-.06,.04),.013,'ivory')
 bar('Stock upper strut',g,(0,.22,-.10),(0,.22,-.38),.025,'ivory');bar('Stock triangular brace',g,(0,.12,-.10),(0,-.025,-.38),.022,'ivory')
 box('Brown stock pad',g,(0,.10,-.40),(.12,.30,.055),'brown')
 for x in [-.108,.108] if heavy else [-.078,.078]:
  for j,(y,z) in enumerate([(.20,-.01),(.17,.03),(.17,-.05)]):
   cyl('Three ivory leaf badge '+str(j),g,(x,y,z),.018,.009,'ivory',axis='x')
common('skill',False)
cyl('Single slender barrel','skill',(0,.16,.49),.035,.62,'steel');tube('Open rifle muzzle','skill',(0,.16,.80),.044,.027,.065,'brown')
cyl('Scope tube','skill',(0,.36,.06),.038,.30,'brown');tube('Scope objective rim','skill',(0,.36,.23),.054,.037,.065,'ivory');cyl('Scope dark glass','skill',(0,.36,.251),.037,.003,'dark')
for z in [-.045,.14]:box('Scope mount','skill',(0,.29,z),(.07,.07,.04),'ivory')
box('Front handgrip','skill',(0,-.02,.25),(.08,.17,.08),'brown')
for x in [-.08,.08]:
 for a,b in [((x,.22,.12),(x,.22,.43)),((x,.22,.12),(x,.09,.27)),((x,.09,.27),(x,.22,.43))]:bar('Skeletal triangle','skill',a,b,.014,'ivory')
cyl('Poison ampoule','skill',(0,-.035,.40),.039,.23,'toxin',axis='y')
for y in [-.155,.09]:cyl('Ampoule collar','skill',(0,y,.40),.047,.035,'ivory',axis='y')
for x in [-.042,.042]:bar('Ampoule protective cage','skill',(x,-.14,.40),(x,.08,.40),.009,'ivory')
common('ultimate',True)
box('Lower support grip','ultimate',(0,-.015,.24),(.11,.16,.09),'brown')
cyl('Rotary rear hub','ultimate',(0,.17,.27),.145,.12,'sage')
for i in range(4):
 a=math.pi/4+i*math.pi/2;x=math.cos(a)*.09;y=.17+math.sin(a)*.09
 cyl('Barrel '+str(i),'rotor',(x,y,.48),.043,.40,'steel');tube('Open bore '+str(i),'rotor',(x,y,.71),.048,.031,.12,'steel');cyl('Recessed dark bore '+str(i),'rotor',(x,y,.665),.030,.004,'dark')
for z in [.33,.65]:tube('Thick ivory barrel ring','rotor',(0,.17,z),.166,.135,.065,'ivory')
for z in [.33,.65]:
 for i in range(8):a=i*math.tau/8;cyl('Ring stud','rotor',(math.cos(a)*.153,.17+math.sin(a)*.153,z+.036),.009,.006,'brown')
for z in [-.07,.17]:bar('Carry handle upright','ultimate',(0,.24,z),(0,.40,z-.06),.025,'ivory')
bar('Brown carry handle','ultimate',(0,.40,-.13),(0,.40,.11),.032,'brown')
tube('Round top sight','ultimate',(0,.39,.25),.051,.038,.025,'ivory')
bar('Sight cross vertical','ultimate',(0,.35,.26),(0,.43,.26),.006,'brown');bar('Sight cross horizontal','ultimate',(-.04,.39,.26),(.04,.39,.26),.006,'brown')
for i in range(8):
 g='belt'+str(i);y=.11-i*.062;x=-.14-.025*math.sin(i*.4);z=.09-.012*i
 cyl('Green dart capsule '+str(i),g,(x,y,z),.020,.11,'toxin');cyl('Ivory belt link '+str(i),g,(x,y,z-.045),.026,.024,'ivory')
 bpy.ops.mesh.primitive_cone_add(vertices=16,radius1=.022,radius2=0,depth=.05,location=xyz((x,y,z+.08)));o=bpy.context.object;o.rotation_euler=(math.pi/2,0,0);finish(o,'Brown dart tip '+str(i),g,'brown')
groups=[]
for group in sorted(set(o['group'] for o in parts)):
 for mat,color in palette.items():
  verts=[];tris=[]
  for o in parts:
   if o['group']!=group or o.data.materials[0].name!=mat:continue
   o.data.calc_loop_triangles()
   for tri in o.data.loop_triangles:
    base=len(verts)
    for idx in tri.vertices:
     p=o.matrix_world@o.data.vertices[idx].co;n=o.matrix_world.to_3x3()@tri.normal
     verts.append([round(v,6) for v in [p.x,p.z,-p.y,n.x,n.z,-n.y]])
    tris.append([base,base+1,base+2])
  if verts:groups.append(dict(group=group,color=[round(c*255) for c in color[:3]],vertices=verts,triangles=tris))
(OUT/'mayo-guns.json').write_text(json.dumps(groups,separators=(',',':')))
# Export independent assets in their runtime grip coordinate system.
for name in ['skill','ultimate']:
 bpy.ops.object.select_all(action='DESELECT')
 for o in parts:o.select_set(o['group']=='skill' if name=='skill' else o['group']!='skill')
 bpy.ops.export_scene.gltf(filepath=str(OUT/f'mayo-{name}-gun.glb'),use_selection=True,export_format='GLB')
for o in parts:o.location.x+=-.65 if o['group']=='skill' else .65
scene=bpy.context.scene;scene.world.color=(.18,.18,.18)
for pos,power in [((1,-3,5),500),((-3,-1,2),350),((0,4,3),500)]:
 bpy.ops.object.light_add(type='AREA',location=pos);o=bpy.context.object;o.data.energy=power;o.data.size=4;o.rotation_euler=(-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(2.8,-3.6,2.4));cam=bpy.context.object;cam.rotation_euler=(Vector((0,-.1,0))-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=2.7;scene.camera=cam
scene.render.engine='CYCLES';scene.cycles.samples=24;scene.cycles.use_denoising=True;scene.render.resolution_x=1200;scene.render.resolution_y=850;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.filepath=str(REVIEW/'guns-model.png');bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'mayo-guns.blend'));bpy.ops.render.render(write_still=True)
print('MAYO_GUNS_OK',sum(len(g['triangles']) for g in groups),flush=True)
