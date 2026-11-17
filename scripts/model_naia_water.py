"""Model the generated water concept as reusable meshes, shared by Blender and Fyrox."""
import bpy,json,math,random
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/effects';REVIEW=ROOT/'artifacts/naia-skill'
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
parts=[]
def tube(name,rows,sides,point,foam):
    vertices=[];triangles=[]
    for i in range(rows+1):
        u=i/rows
        for j in range(sides+1):
            v=j/sides;x,y,z=point(u,v*math.tau)
            vertices.append([x,y,z,u,v])
    for i in range(rows):
        for j in range(sides):
            a=i*(sides+1)+j;b=a+sides+1
            triangles.extend([[a,b,a+1],[a+1,b,b+1]])
    parts.append(dict(name=name,foam=foam,vertices=vertices,triangles=triangles))
def center(u):return (.028*math.sin(u*20)*u,.036*math.sin(u*14+.7)*u)
def body(u,a):
    x,y=center(u);r=(.035+.13*math.sqrt(u))*(1+.13*math.sin(u*55+a*3)+.08*math.sin(u*97-a*2))
    return x+r*math.cos(a),y+r*math.sin(a),u
tube('Translucent continuous water',100,20,body,False)
for k in range(3):
    def ribbon(u,a,k=k):
        x,y=center(u);theta=u*19+k*math.tau/3;r=(.04+.15*math.sqrt(u))
        width=(.009+.014*u)*(1+.25*math.sin(u*91+k))
        return x+(r+width*math.cos(a))*math.cos(theta),y+(r+width*math.cos(a))*math.sin(theta),u+width*.09*math.sin(a)
    tube(f'Flowing foam ribbon {k+1}',100,6,ribbon,True)
random.seed(17)
for k in range(24):
    u=random.uniform(.12,.99);theta=random.uniform(0,math.tau);r=random.uniform(.21,.38)*math.sqrt(u);size=random.uniform(.012,.030)
    x,y=center(u);x+=r*math.cos(theta);y+=r*math.sin(theta)
    def drop(v,a,u=u,x=x,y=y,size=size):
        lat=math.pi*v
        return x+size*math.sin(lat)*math.cos(a),y+size*math.sin(lat)*math.sin(a),u+size*.6*math.cos(lat)
    tube(f'Spray droplet {k+1}',6,8,drop,k%4==0)
# A flared broken crown gives the distant end a splash instead of a flat cap.
for k in range(7):
    theta=k*math.tau/7
    def splash(u,a,theta=theta):
        angle=theta+.25*u;r=.13+.30*math.sin(u*1.5);thick=.025*(1-u)+.006
        return (r+thick*math.cos(a))*math.cos(angle),(r+thick*math.cos(a))*math.sin(angle),.965+.03*math.sin(u*2.8)+thick*.12*math.sin(a)
    tube(f'Splash crown {k+1}',14,6,splash,True)
(OUT/'naia-water-mesh.json').write_text(json.dumps(parts,separators=(',',':')))
materials=[]
for name,color,metal,rough,alpha in [('Aqua liquid',(.025,.47,.67,1),.35,.18,.72),('White foam',(.66,.94,1,1),.1,.28,1)]:
    mat=bpy.data.materials.new(name);mat.diffuse_color=color;mat.use_nodes=True
    bsdf=mat.node_tree.nodes.get('Principled BSDF');bsdf.inputs['Base Color'].default_value=color;bsdf.inputs['Metallic'].default_value=metal;bsdf.inputs['Roughness'].default_value=rough;bsdf.inputs['Alpha'].default_value=alpha
    if not alpha==1:bsdf.inputs['Transmission Weight'].default_value=.25
    materials.append(mat)
for part in parts:
    mesh=bpy.data.meshes.new(part['name']);mesh.from_pydata([(v[2]*7-3.5,v[0],v[1]) for v in part['vertices']],[],part['triangles']);mesh.update()
    obj=bpy.data.objects.new(part['name'],mesh);bpy.context.collection.objects.link(obj);obj.data.materials.append(materials[int(part['foam'])])
    for p in mesh.polygons:p.use_smooth=True
    uv=mesh.uv_layers.new(name='Flow UV')
    for loop in mesh.loops:uv.data[loop.index].uv=part['vertices'][loop.vertex_index][3:5]
scene=bpy.context.scene;scene.world.color=(.065,.08,.11)
for position,power,size in [((0,-3,5),1100,6),((0,4,3),900,5),((-4,0,2),600,3)]:
    bpy.ops.object.light_add(type='AREA',location=position);light=bpy.context.object;light.data.energy=power;light.data.size=size;light.rotation_euler=(-light.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(1,-9,3));camera=bpy.context.object;camera.rotation_euler=(-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=8;scene.camera=camera
scene.render.engine='CYCLES';scene.cycles.samples=24;scene.cycles.use_denoising=True
scene.render.resolution_x=1400;scene.render.resolution_y=600;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.filepath=str(REVIEW/'water-model.png')
bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'naia-healing-stream.blend'))
bpy.ops.export_scene.gltf(filepath=str(OUT/'naia-healing-stream.glb'),export_format='GLB',export_cameras=False,export_lights=False)
bpy.ops.render.render(write_still=True)
print('NAIA_WATER_MODEL_OK',sum(len(p['vertices']) for p in parts),sum(len(p['triangles']) for p in parts),flush=True)
