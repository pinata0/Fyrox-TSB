"""Editable faceted Quicksilver Lance from vivi-lance-concept.png, with runtime geometry."""
import bpy,json,math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/effects';REVIEW=ROOT/'artifacts/vivi-skills';REVIEW.mkdir(parents=True,exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
palette={'silver':(.72,.75,.80,1),'light':(.93,.95,.98,1),'shade':(.30,.32,.39,1),'edge':(.17,.075,.105,1)};mats={};parts=[]
for name,c in palette.items():
 m=bpy.data.materials.new(name);m.diffuse_color=c;m.use_nodes=True;p=m.node_tree.nodes['Principled BSDF'];p.inputs['Base Color'].default_value=c;p.inputs['Metallic'].default_value=.78;p.inputs['Roughness'].default_value=.24;mats[name]=m
def finish(o,name,mat):o.name=name;o.data.materials.append(mats[mat]);o['palette']=mat;parts.append(o);return o
def mesh(name,verts,faces,mat):
 d=bpy.data.meshes.new(name);d.from_pydata(verts,[],faces);d.update();o=bpy.data.objects.new(name,d);bpy.context.collection.objects.link(o);return finish(o,name,mat)
def tube(name,points,r,mat):
 c=bpy.data.curves.new(name,'CURVE');c.dimensions='3D';c.bevel_depth=r;c.bevel_resolution=2;s=c.splines.new('POLY');s.points.add(len(points)-1)
 for p,v in zip(s.points,points):p.co=(*v,1)
 o=bpy.data.objects.new(name,c);bpy.context.collection.objects.link(o);bpy.context.view_layer.objects.active=o;o.select_set(True);bpy.ops.object.convert(target='MESH');o.select_set(False);return finish(o,name,mat)
def cylinder(name,z,depth,r,mat,r2=None):
 bpy.ops.mesh.primitive_cone_add(vertices=12,radius1=r,radius2=r if r2 is None else r2,depth=depth,location=(0,0,z));return finish(bpy.context.object,name,mat)
def diamond(name,z,rx,rz,mat):
 v=[(0,0,z+rz),(rx,0,z),(0,rx*.6,z),(-rx,0,z),(0,-rx*.6,z),(0,0,z-rz)]
 return mesh(name,v,[(0,i,1+i%4) for i in range(1,5)]+[(5,1+i%4,i) for i in range(1,5)],mat)
# Long pointed leaf and swept lower shoulder barbs, with a narrow ridge instead of a flat card.
outline=[(0,1.75),(.105,1.15),(.085,1.10),(.11,1.06),(.23,1.055),(.145,.985),(.11,1.015),(.055,.95),(.026,.88),(-.026,.88),(-.055,.95),(-.11,1.015),(-.145,.985),(-.23,1.055),(-.11,1.06),(-.085,1.10),(-.105,1.15)]
for front in [-1,1]:
 boundary=[(x,front*.013,z) for x,z in outline];ridge=(0,front*.067,1.105)
 for i in range(len(outline)):
  j=(i+1)%len(outline);a=boundary[i];b=boundary[j]
  material='light' if (front==-1 and (a[0]+b[0])<0) else 'silver' if front==-1 else 'shade'
  verts=[a,b,ridge];faces=[(0,1,2)] if front==-1 else [(2,1,0)]
  mesh('Faceted spearhead '+str(front)+' '+str(i),verts,faces,material)
for i in range(len(outline)):
 j=(i+1)%len(outline);x,z=outline[i];xx,zz=outline[j]
 mesh('Burgundy steel edge '+str(i),[(x,-.013,z),(xx,-.013,zz),(xx,.013,zz),(x,.013,z)],[(0,1,2,3)],'edge')
 tube('Fine dark rim '+str(i),[(x,-.014,z),(xx,-.014,zz)],.002,'edge')
for front in [-1,1]:
 mesh('Recessed diamond channel '+str(front),[(0,front*.027,1.60),(.014,front*.069,1.13),(0,front*.048,.98),(-.014,front*.069,1.13)],[(0,1,2),(0,2,3)],'edge')
cylinder('Long mercury shaft',.03,1.67,.018,'silver')
cylinder('Lower grip end',-.815,.035,.024,'shade')
diamond('Pointed diamond butt',-.91,.041,.090,'silver')
cylinder('Head collar lower',.84,.035,.034,'shade')
cylinder('Head collar taper',.87,.035,.044,'silver',.026)
cylinder('Head collar flare',.845,.030,.027,'light',.044)
for k in range(3):
 tube('Spiral silver binding '+str(k),[(.022*math.cos(i*math.tau/40),.022*math.sin(i*math.tau/40),.79-k*.045-i/40*.045) for i in range(41)],.005,'light')
bpy.context.view_layer.update();groups=[]
for mat,color in palette.items():
 verts=[];triangles=[]
 for o in parts:
  if o['palette']!=mat:continue
  o.data.calc_loop_triangles()
  for tri in o.data.loop_triangles:
   base=len(verts)
   for idx in tri.vertices:
    p=o.matrix_world@o.data.vertices[idx].co;n=o.matrix_world.to_3x3()@tri.normal
    verts.append([round(v,6) for v in [p.x,p.z,-p.y,n.x,n.z,-n.y]])
   triangles.append([base,base+1,base+2])
 if verts:groups.append(dict(group=mat,color=[round(c*255) for c in color[:3]],vertices=verts,triangles=triangles))
(OUT/'vivi-lance-mesh.json').write_text(json.dumps(groups,separators=(',',':')))
bpy.ops.object.select_all(action='DESELECT')
for o in parts:o.select_set(True)
bpy.ops.export_scene.gltf(filepath=str(OUT/'vivi-lance.glb'),export_format='GLB',use_selection=True)
scene=bpy.context.scene;scene.world.color=(.3,.3,.3)
for pos,power in [((1,-3,4),600),((-2,-2,2),300),((0,3,2),700)]:
 bpy.ops.object.light_add(type='AREA',location=pos);o=bpy.context.object;o.data.energy=power;o.data.size=3;o.rotation_euler=(Vector((0,0,.4))-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(1.8,-5,1.0));cam=bpy.context.object;cam.rotation_euler=(Vector((0,0,.38))-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=3.1;scene.camera=cam
scene.render.engine='CYCLES';scene.cycles.samples=24;scene.cycles.use_denoising=True;scene.render.resolution_x=700;scene.render.resolution_y=1000;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.filepath=str(REVIEW/'lance-model.png');bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'vivi-lance.blend'));bpy.ops.render.render(write_still=True)
print('VIVI_LANCE_OK',sum(len(g['triangles']) for g in groups),flush=True)
