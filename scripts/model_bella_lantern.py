"""Build the Bella concept as editable geometry, GLB and engine-ready material groups."""
import bpy, json, math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/effects';REVIEW=ROOT/'artifacts/bella-skills'
REVIEW.mkdir(parents=True,exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
palette={'indigo':(.075,.055,.15,1),'violet':(.25,.13,.39,1),'silver':(.66,.57,.82,1),'crystal':(.47,.13,.92,1),'flame':(.20,.64,1.0,1),'core':(.70,1.0,1.0,1)}
mats={};parts=[]
for name,c in palette.items():
 m=bpy.data.materials.new(name);m.diffuse_color=c;m.use_nodes=True;p=m.node_tree.nodes['Principled BSDF'];p.inputs['Base Color'].default_value=c;p.inputs['Roughness'].default_value=.35;p.inputs['Metallic'].default_value=.55 if name not in ['flame','core','crystal'] else .1
 if name in ['flame','core','crystal']:p.inputs['Emission Color'].default_value=c;p.inputs['Emission Strength'].default_value=2.5 if name=='core' else 1.0
 mats[name]=m

# Native Blender Z-up. Export grip origin at the top of the arch (0,0,0).
def finish(o,name,mat):
 o.name=name;o.data.materials.append(mats[mat]);o['palette']=mat;parts.append(o);return o
def tube(name,points,r,mat,closed=False):
 curve=bpy.data.curves.new(name,'CURVE');curve.dimensions='3D';curve.bevel_depth=r;curve.bevel_resolution=2;curve.resolution_u=2
 spl=curve.splines.new('POLY');spl.points.add(len(points)-1)
 for p,v in zip(spl.points,points):p.co=(*v,1)
 spl.use_cyclic_u=closed;o=bpy.data.objects.new(name,curve);bpy.context.collection.objects.link(o);bpy.context.view_layer.objects.active=o;o.select_set(True);bpy.ops.object.convert(target='MESH');o.select_set(False);return finish(o,name,mat)
def mesh(name,verts,faces,mat):
 data=bpy.data.meshes.new(name);data.from_pydata(verts,[],faces);data.update();o=bpy.data.objects.new(name,data);bpy.context.collection.objects.link(o);return finish(o,name,mat)
def ring(name,z,r,mat,thick=.006,n=6):
 return tube(name,[(r*math.cos(i*math.tau/n),r*math.sin(i*math.tau/n),z) for i in range(n)],thick,mat,True)
def gem(name,p,size,mat):
 x,y,z=p;rx,ry,rz=size;verts=[(x+rx,y,z),(x,y+ry,z),(x-rx,y,z),(x,y-ry,z),(x,y,z+rz),(x,y,z-rz)]
 return mesh(name,verts,[(i,(i+1)%4,4) for i in range(4)]+[((i+1)%4,i,5) for i in range(4)],mat)
for z,r in [(-.20,.115),(-.43,.105)]:
 ring('Hexagonal cage rim',z,r,'indigo',.012);ring('Silver rim inlay',z+.008,r+.006,'silver',.003)
for i in range(6):
 a=i*math.tau/6
 tube('Curved open cage rib '+str(i),[(math.cos(a)*(.105+.010*math.sin(t*math.pi)),math.sin(a)*(.105+.010*math.sin(t*math.pi)),-.43+.23*t) for t in [j/16 for j in range(17)]],.005,'silver')
 gem('Lower amethyst setting',(.107*math.cos(a),.107*math.sin(a),-.428),(.018,.018,.027),'violet')
 gem('Silver lower star',(.113*math.cos(a),.113*math.sin(a),-.428),(.008,.008,.017),'silver')
 # Six petal roof segments taper to the crown, with scalloped tips.
 verts=[]
 for row in range(9):
  t=row/8;r=.018+.135*t;z=-.116-.086*t+.022*t**5
  for col in range(9):
   q=col/8;b=a+(q-.5)*math.tau/6
   zz=z-.018*math.sin(q*math.pi)*t
   verts.append((r*math.cos(b),r*math.sin(b),zz))
 faces=[]
 for row in range(8):
  for col in range(8):j=row*9+col;faces.append((j,j+1,j+10,j+9))
 roof=mesh('Scalloped petal roof '+str(i),verts,faces,'violet' if i%2 else 'indigo');solid=roof.modifiers.new('Metal thickness','SOLIDIFY');solid.thickness=.003
 bpy.context.view_layer.objects.active=roof;bpy.ops.object.modifier_apply(modifier=solid.name)
 tube('Petal silver seam',verts[::9],.0025,'silver')
 tube('Petal silver lip',verts[-9:],.003,'silver')
 gem('Roof amethyst',(.085*math.cos(a),.085*math.sin(a),-.160),(.012,.012,.018),'crystal')
ring('Foot',-.447,.067,'indigo',.010)
for i in range(6):
 a=i*math.tau/6;tube('Base taper',[(.10*math.cos(a),.10*math.sin(a),-.435),(0,0,-.482)],.005,'violet')
tube('Crystal bail',[(.011*math.cos(i*math.tau/24),0,-.491+.014*math.sin(i*math.tau/24)) for i in range(24)],.0025,'silver',True)
gem('Hanging violet crystal',(0,0,-.529),(.017,.017,.031),'crystal')
# Arched handle. Its top is the wrist contact point.
tube('Carrying arch',[(.115*math.cos(t),0,-.115+.115*math.sin(t)) for t in [math.pi*i/40 for i in range(41)]],.006,'indigo')
for x in [-.115,.115]:
 tube('Handle link',[(x+.009*math.cos(i*math.tau/24),0,-.133+.015*math.sin(i*math.tau/24)) for i in range(24)],.003,'silver',True)
gem('Crown star',(0,0,-.114),(.020,.013,.024),'silver')
# Open crescent, an explicit band with pointed ends.
outer=[];inner=[]
for i in range(33):
 t=math.radians(55)+math.radians(250)*i/32
 outer.append((.032*math.cos(t),-.002,-.071+.032*math.sin(t)))
 inner.append((.014+.028*math.cos(t),-.002,-.069+.028*math.sin(t)))
v=[]
for a,b in zip(outer,inner):v.extend([a,b])
crescent=mesh('Silver crescent',v,[(i*2,i*2+1,i*2+3,i*2+2) for i in range(32)],'silver')
solid=crescent.modifiers.new('Crescent depth','SOLIDIFY');solid.thickness=.006;bpy.context.view_layer.objects.active=crescent;bpy.ops.object.modifier_apply(modifier=solid.name)

def flame(name,scale,mat,phase):
 verts=[];rows=20;cols=16
 for j in range(rows+1):
  t=j/rows;rad=(math.sin(math.pi*t)**.65)*.041*scale
  curl=.031*t*t*math.sin(t*6+phase);z=-.407+.182*t*scale
  for k in range(cols):
   a=k*math.tau/cols;verts.append((curl+rad*math.cos(a),rad*math.sin(a),z))
 faces=[]
 for j in range(rows):
  for k in range(cols):n=j*cols+k;next=j*cols+(k+1)%cols;faces.append((n,next,next+cols,n+cols))
 return mesh(name,verts,faces,mat)
flame('Swirling cyan soul',1,'flame',0);flame('White hot inner core',.65,'core',1.5)
for i in range(3):
 a=i*math.tau/3;tube('Violet flame curl',[(.04*math.cos(a+t*4)*(1-t),.04*math.sin(a+t*4)*(1-t),-.36+t*.16) for t in [j/24 for j in range(25)]],.004,'crystal')
bpy.context.view_layer.update()
groups=[]
for mat,color in palette.items():
 verts=[];tris=[]
 for o in parts:
  if o['palette']!=mat:continue
  o.data.calc_loop_triangles()
  for tri in o.data.loop_triangles:
   base=len(verts)
   for idx in tri.vertices:
    p=o.matrix_world@o.data.vertices[idx].co;n=o.matrix_world.to_3x3()@tri.normal
    verts.append([round(v,6) for v in [p.x,p.z,-p.y,n.x,n.z,-n.y]])
   tris.append([base,base+1,base+2])
 if verts:groups.append(dict(group=mat,color=[round(c*255) for c in color[:3]],vertices=verts,triangles=tris))
(OUT/'bella-lantern-mesh.json').write_text(json.dumps(groups,separators=(',',':')))
bpy.ops.object.select_all(action='DESELECT')
for o in parts:o.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(OUT/'bella-lantern.glb'),export_format='GLB',use_selection=True)
scene=bpy.context.scene;scene.world.color=(.18,.18,.18)
for pos,power in [((1,-2,2),160),((-2,-1,1),100),((0,2,1),180)]:
 bpy.ops.object.light_add(type='AREA',location=pos);o=bpy.context.object;o.data.energy=power;o.data.size=2;o.rotation_euler=(Vector((0,0,-.25))-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(.8,-1.2,.35));cam=bpy.context.object;cam.rotation_euler=(Vector((0,0,-.27))-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=.72;scene.camera=cam
scene.render.engine='CYCLES';scene.cycles.samples=32;scene.cycles.use_denoising=True;scene.render.resolution_x=1000;scene.render.resolution_y=1000;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.filepath=str(REVIEW/'lantern-model.png');bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'bella-lantern.blend'));bpy.ops.render.render(write_still=True)
print('BELLA_LANTERN_OK',sum(len(g['triangles']) for g in groups),flush=True)
