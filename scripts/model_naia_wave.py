"""Build the generated curling wave as an independent Blender/GLB/runtime asset."""
import bpy, json, math, random
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/effects';REVIEW=ROOT/'artifacts/naia-ultimate'
REVIEW.mkdir(parents=True,exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
parts=[]
def grid(name,rows,cols,point,foam=False):
    vertices=[[*point(i/rows,j/cols),i/rows,j/cols] for i in range(rows+1) for j in range(cols+1)]
    triangles=[]
    for i in range(rows):
        for j in range(cols):
            a=i*(cols+1)+j;b=a+cols+1
            triangles.extend([[a,b,a+1],[a+1,b,b+1]])
    parts.append(dict(name=name,foam=foam,vertices=vertices,triangles=triangles))
def profile(u,v):
    x=(u-.5)*10
    # Low trailing skirt rises into a tall, forward-overhanging crest.
    angle=v*math.pi*1.13
    y=3.8*math.sin(angle*.70)
    z=-2.3+2.55*v+1.05*math.sin(angle-1.0)
    y*=max(.06,math.sin(u*math.pi))**.20
    y+=.09*math.sin(x*4.8+v*12)*math.sin(v*math.pi)
    z+=.10*math.sin(x*3.1+v*7)*v
    return x,max(.025,y),z
grid('Continuous curling aqua water wall',100,38,profile)
# One irregular blanket of whitewater follows the curl; scalloped torn tips
# replace the cylindrical rails that make a wave read as a rigid object.
def whitewater(u,v):
    start=.55+.045*math.sin(u*67)+.025*math.sin(u*131)
    end=.92+.07*math.sin(u*91)**2
    t=start+(end-start)*v
    x,y,z=profile(u,t)
    lump=.025+.055*math.sin(u*97+v*15)**2
    return x,y+lump*.5,z+lump
grid('Scalloped breaking whitewater',180,16,whitewater,True)
for k in range(20):
    def streak(u,v,k=k):
        across=(k+.5)/20+.007*math.sin(u*8+k)
        x,y,z=profile(across,.10+.83*u)
        r=(.007+.018*u)*(0.55+0.45*math.sin(u*23+k)**2);a=v*math.tau
        return x+r*math.cos(a),y+.018*math.sin(a),z+.028+r*math.sin(a)
    grid(f'Foam flowing up the face {k+1}',28,5,streak,True)
random.seed(29)
for k in range(50):
    x,y,z=profile(random.random(),random.uniform(.82,1))
    y+=random.uniform(.05,.55);z+=random.uniform(-.18,.50);r=random.uniform(.025,.075)
    def drop(u,v,x=x,y=y,z=z,r=r):
        return x+r*math.sin(u*math.pi)*math.cos(v*math.tau),y+r*1.4*math.cos(u*math.pi),z+r*math.sin(u*math.pi)*math.sin(v*math.tau)
    grid(f'Spray {k+1}',6,8,drop,k%3==0)
grid('Shallow trailing water sheet',70,12,lambda u,v:((u-.5)*10,.035+.02*math.sin(u*75+v*9),-3.4+v*3.9))
grid('Broken front foam skirt',100,6,lambda u,v:((u-.5)*10,.045+.10*math.sin(v*math.pi),.32+.35*v+.13*math.sin(u*84)),True)
(OUT/'naia-ultimate-wave-mesh.json').write_text(json.dumps(parts,separators=(',',':')))
materials=[]
for name,color,rough in [('Turquoise water',(.018,.46,.63,1),.19),('White rolling foam',(.73,.96,1,1),.30)]:
    m=bpy.data.materials.new(name);m.diffuse_color=color;m.use_nodes=True
    b=m.node_tree.nodes.get('Principled BSDF');b.inputs['Base Color'].default_value=color;b.inputs['Roughness'].default_value=rough
    b.inputs['Metallic'].default_value=.18
    if len(materials)==0:b.inputs['Transmission Weight'].default_value=.18
    materials.append(m)
for part in parts:
    m=bpy.data.meshes.new(part['name']);m.from_pydata([(v[0],-v[2],v[1]) for v in part['vertices']],[],part['triangles']);m.update()
    o=bpy.data.objects.new(part['name'],m);bpy.context.collection.objects.link(o);m.materials.append(materials[int(part['foam'])])
    for p in m.polygons:p.use_smooth=True
    uv=m.uv_layers.new(name='Wave flow')
    for loop in m.loops:uv.data[loop.index].uv=part['vertices'][loop.vertex_index][3:5]
scene=bpy.context.scene;scene.world.color=(.06,.08,.11)
for pos,power,size in [((1,-5,8),1900,7),((-5,0,5),1300,5),((0,6,6),2200,6)]:
    bpy.ops.object.light_add(type='AREA',location=pos);o=bpy.context.object;o.data.energy=power;o.data.size=size;o.rotation_euler=(Vector((0,0,1.8))-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(11,-13,7));camera=bpy.context.object;camera.rotation_euler=(Vector((0,0,1.7))-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=13.5;scene.camera=camera
scene.render.engine='CYCLES';scene.cycles.samples=24;scene.cycles.use_denoising=True
scene.render.resolution_x=1400;scene.render.resolution_y=800;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.filepath=str(REVIEW/'wave-model.png')
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'naia-ultimate-wave.blend'))
bpy.ops.export_scene.gltf(filepath=str(OUT/'naia-ultimate-wave.glb'),export_format='GLB',export_cameras=False,export_lights=False)
bpy.ops.render.render(write_still=True)
print('NAIA_WAVE_MODEL_OK',sum(len(p['vertices']) for p in parts),flush=True)
