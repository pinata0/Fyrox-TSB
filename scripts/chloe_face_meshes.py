"""Small skinned geometric expression overlays; no replacement portrait textures."""
import bpy,math
from mathutils import Vector
from mathutils.bvhtree import BVHTree
def build_faces(rig,body,bear):
 def mat(hex):
  m=bpy.data.materials.new('Color_'+hex+'_ChloeExpression');m.diffuse_color=tuple(int(hex[i:i+2],16)/255 for i in (0,2,4))+(1,);m.use_nodes=True;m.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=m.diffuse_color;m.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.85;return m
 ink=mat('533740');skin=mat('f6e4ce');white=mat('f5f2ef')
 def surface(obj):
  obj.data.calc_loop_triangles();return BVHTree.FromPolygons([v.co for v in obj.data.vertices],[t.vertices for t in obj.data.loop_triangles],all_triangles=True)
 bvhs={body.name:surface(body),bear.name:surface(bear)}
 def point(obj,x,z,offset=.006):
  hit,*_=bvhs[obj.name].ray_cast(Vector((x,-2,z)),Vector((0,1,0)),4)
  return Vector((x,hit.y-offset if hit else -.28,z))
 def shape(name,obj,bone,centers,mode):
  verts=[];faces=[];indices=[]
  for cx,cz,rx,rz in centers:
   # An oval lid follows the model surface, fully covering its original painted eye.
   if mode in ['scissor','closed','angry']:
    base=len(verts);verts.append(point(obj,cx,cz,.012));previous=None
    for ring in range(1,9):
     current=[]
     for i in range(32):
      a=i*math.tau/32;current.append(len(verts));verts.append(point(obj,cx+math.cos(a)*rx*ring/8,cz+math.sin(a)*rz*ring/8,.012))
     for i in range(32):
      faces.append((base,current[i],current[(i+1)%32]) if previous is None else (previous[i],current[i],current[(i+1)%32],previous[(i+1)%32]));indices.append(0)
     previous=current
   if mode=='angry':
    # Narrow white eye and a small pupil below a sloped lid, as in the move clip.
    for radius_x,radius_z,offset_x,material_index in [(rx*.72,rz*.29,0,2),(rx*.17,rz*.22,-rx*.16 if cx<0 else rx*.16,1)]:
     base=len(verts);verts.append(point(obj,cx+offset_x,cz-rz*.22,.020))
     for i in range(25):
      a=i*math.tau/24;verts.append(point(obj,cx+offset_x+math.cos(a)*radius_x,cz-rz*.22+math.sin(a)*radius_z,.020))
     for i in range(24):faces.append((base,base+i+1,base+i+2));indices.append(material_index)
   sign=1 if cx<0 else -1
   segments=([(-.8,.45),(.4,0),(-.8,-.45)] if mode=='scissor' else [(-.85,.38),(.8,-.25)] if mode=='angry' else [(-.8,0),(0,-.12),(.8,0)])
   for a,b in zip(segments,segments[1:]):
    a=Vector((cx+sign*a[0]*rx,cz+a[1]*rz));b=Vector((cx+sign*b[0]*rx,cz+b[1]*rz));d=(b-a).normalized();normal=Vector((-d.y,d.x))*(.009 if obj==body else .016)
    base=len(verts)
    for q in [a-normal,b-normal,b+normal,a+normal]:verts.append(point(obj,q.x,q.y,.026))
    faces.append((base,base+1,base+2,base+3));indices.append(1)
  me=bpy.data.meshes.new(name);me.from_pydata(verts,[],faces);me.update();o=bpy.data.objects.new(name,me);bpy.context.collection.objects.link(o);o.data.materials.append(white if obj==bear else skin);o.data.materials.append(ink);o.data.materials.append(white)
  for p,i in zip(me.polygons,indices):p.material_index=i;p.use_smooth=True
  normals=[]
  for v in verts:
   hit,normal,*_=bvhs[obj.name].ray_cast(Vector((v.x,-2,v.z)),Vector((0,1,0)),4)
   normals.append(normal if normal else Vector((0,-1,0)))
  me.normals_split_custom_set_from_vertices(normals)
  o.parent=rig;g=o.vertex_groups.new(name=bone);g.add(list(range(len(verts))),1,'REPLACE');m=o.modifiers.new('Expression follows head','ARMATURE');m.object=rig
  o.hide_render=True
 shape('ChloeScissorEyes',body,'Head',[(-.146,1.187,.080,.081),(.128,1.187,.080,.081)],'scissor')
 shape('ChloeMoveEyes',body,'Head',[(-.146,1.187,.080,.081),(.128,1.187,.080,.081)],'angry')
 shape('SebastianClosedEyes',bear,'Sebastian.Head',[(-.28,1.95,.115,.105),(.185,1.95,.115,.105)],'closed')
