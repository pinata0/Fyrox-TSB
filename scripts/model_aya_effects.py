"""Model the generated snow butterfly, white blossoms and a faceted ice prison.
Exports editable Blender/GLB and the exact same mesh arrays consumed by Fyrox.
"""
import bpy,json,math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/effects';WORK=ROOT/'artifacts/aya-skills';WORK.mkdir(exist_ok=True)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
parts=[]
def grid(asset,name,group,style,rows,cols,point):
    vertices=[[*point(i/rows,j/cols),i/rows,j/cols] for i in range(rows+1) for j in range(cols+1)];triangles=[]
    for i in range(rows):
        for j in range(cols):
            a=i*(cols+1)+j;b=a+cols+1;triangles.extend([[a,b,a+1],[a+1,b,b+1]])
    parts.append(dict(asset=asset,name=name,group=group,style=style,vertices=vertices,triangles=triangles))
def tube(asset,name,group,style,path,r):
    grid(asset,name,group,style,12,5,lambda u,v:tuple(Vector(path(u))+Vector((math.cos(v*math.tau)*r,math.sin(v*math.tau)*r,0))))
def orb(asset,name,group,style,center,scale):
    grid(asset,name,group,style,8,12,lambda u,v:(center[0]+scale[0]*math.sin(u*math.pi)*math.cos(v*math.tau),center[1]+scale[1]*math.cos(u*math.pi),center[2]+scale[2]*math.sin(u*math.pi)*math.sin(v*math.tau)))
def bezier(points,t):
    return tuple((1-t)**3*points[0][k]+3*(1-t)**2*t*points[1][k]+3*(1-t)*t*t*points[2][k]+t**3*points[3][k] for k in range(2))
upper=[[(.025,.035),(.14,.21),(.34,.37),(.46,.32)],[(.46,.32),(.39,.27),(.45,.21),(.40,.15)],[(.40,.15),(.46,.05),(.29,-.005),(.025,.035)]]
lower=[[(.02,.0),(.17,.04),(.39,-.04),(.31,-.18)],[(.31,-.18),(.27,-.23),(.22,-.15),(.23,-.30)],[(.23,-.30),(.14,-.24),(.065,-.12),(.02,.0)]]
for side in [-1,1]:
    group=0 if side<0 else 1
    for kind,outline in [('upper',upper),('lower',lower)]:
        def edge(t):
            q=min(2,int(t*3));return bezier(outline[q],min(1,t*3-q))
        def wing(u,v):
            x,z=edge(v);return (side*(.02+(x-.02)*u),.035*math.sin(u*math.pi)*math.sin(v*math.pi),z*u)
        grid('butterfly',f'{side} {kind} pearlescent membrane',group,0,5,48,wing)
        tube('butterfly',f'{side} {kind} lavender rim',group,1,lambda t:(side*edge(t)[0],.004,edge(t)[1]),.006)
        for j in range(1,7):
            x,z=edge(j/8)
            tube('butterfly',f'{side} {kind} vein {j}',group,2,lambda t,x=x,z=z:(side*(.025+(x-.025)*t),.006+.028*math.sin(t*math.pi),z*t),.0025)
            orb('butterfly',f'{side} {kind} pearl {j}',group,2,(side*x*.89,.012,z*.89),(.012,.004,.009))
    tube('butterfly',f'Antenna {side}',2,1,lambda t:(side*(.018+.095*t),.02+.08*t*t,.125+.16*t),.003)
orb('butterfly','Violet tapered body',2,3,(0,0,-.015),(.023,.028,.15))
orb('butterfly','Violet head',2,3,(0,0,.135),(.028,.028,.028))
for j in range(5):
    angle=j*math.tau/5
    def petal(u,v,angle=angle):
        r=.025+u*.32;w=math.sin(u*math.pi)**.7*.115*(2*v-1)
        return (math.cos(angle)*r-math.sin(angle)*w,.16+.12*u*u+.04*(2*v-1)**2,math.sin(angle)*r+math.cos(angle)*w)
    grid('flower',f'White cupped petal {j}',0,4,12,10,petal)
orb('flower','Pale gold flower heart',0,5,(0,.175,0),(.045,.027,.045))
tube('flower','Ice-blue stem',0,2,lambda t:(0,.17*t,0),.012)
for j in range(8):
    a=j*math.tau/8
    tube('flower',f'Stamen {j}',0,5,lambda t,a=a:(math.cos(a)*.055*t,.175+.07*t,math.sin(a)*.055*t),.003)
    orb('flower',f'Stamen tip {j}',0,5,(math.cos(a)*.055,.245,math.sin(a)*.055),(.009,.009,.009))
# Closed beveled hexagonal crystal, with transparent facets and bright edges.
rings=[(.0,.55),(.22,.78),(1.55,.70),(2.12,.46),(2.38,.0)];vertices=[]
for y,r in rings:
    for j in range(6):
        a=j*math.tau/6+.15;vertices.append([math.cos(a)*r,y,math.sin(a)*r,y/2.38,j/6])
triangles=[]
for k in range(4):
    for j in range(6):
        a=k*6+j;b=k*6+(j+1)%6;triangles.extend([[a,b,a+6],[b,b+6,a+6]])
triangles.extend([[0,j+1,j] for j in range(1,5)])
parts.append(dict(asset='ice',name='Faceted translucent ice prison',group=0,style=6,vertices=vertices,triangles=triangles))
for j in range(6):
    for k in range(4):
        a=Vector(vertices[k*6+j][:3]);b=Vector(vertices[(k+1)*6+j][:3])
        tube('ice',f'Frost edge {j}/{k}',0,2,lambda t,a=a,b=b:tuple(a.lerp(b,t)),.012)
(OUT/'aya-effects-mesh.json').write_text(json.dumps(parts,separators=(',',':')))
palette=[(.61,.76,1,1),(.58,.23,1,1),(.58,.91,1,1),(.24,.08,.45,1),(.94,.98,1,1),(1,.91,.61,1),(.18,.60,.86,.28)]
materials=[]
for j,color in enumerate(palette):
    mat=bpy.data.materials.new(['Pearl wing','Violet glow','Frost glow','Violet body','White petal','Warm pollen','Clear ice'][j]);mat.diffuse_color=color;mat.use_nodes=True
    bsdf=mat.node_tree.nodes.get('Principled BSDF');bsdf.inputs['Base Color'].default_value=color;bsdf.inputs['Roughness'].default_value=.25
    if j in [1,2]:bsdf.inputs['Emission Color'].default_value=color;bsdf.inputs['Emission Strength'].default_value=1.0
    if j==6:bsdf.inputs['Transmission Weight'].default_value=.7;bsdf.inputs['Alpha'].default_value=.32
    materials.append(mat)
for asset,offset in [('butterfly',(0,0,1.1)),('flower',(-1.25,0,0)),('ice',(1.65,0,0))]:
    root=bpy.data.objects.new('Aya '+asset,None);bpy.context.collection.objects.link(root);root.location=offset
    for part in [p for p in parts if p['asset']==asset]:
        mesh=bpy.data.meshes.new(part['name']);mesh.from_pydata([(v[0],-v[2],v[1]) for v in part['vertices']],[],part['triangles']);mesh.update()
        obj=bpy.data.objects.new(part['name'],mesh);bpy.context.collection.objects.link(obj);obj.parent=root;mesh.materials.append(materials[part['style']])
        uv=mesh.uv_layers.new(name='Pearl gradient')
        for loop in mesh.loops:uv.data[loop.index].uv=part['vertices'][loop.vertex_index][3:5]
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=20;scene.render.resolution_x=1200;scene.render.resolution_y=800;scene.render.resolution_percentage=100
scene.world.color=(.08,.10,.16);scene.view_settings.view_transform='Standard'
target=Vector((.5,0,1.0))
for loc,power in [((0,-4,6),550),((-4,1,4),400),((4,4,5),700)]:
    bpy.ops.object.light_add(type='AREA',location=loc);o=bpy.context.object;o.data.energy=power;o.data.size=4;o.rotation_euler=(target-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(1,-6,5));camera=bpy.context.object;camera.rotation_euler=(target-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=4.8;scene.camera=camera
bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'aya-effects.blend'))
bpy.ops.export_scene.gltf(filepath=str(OUT/'aya-effects.glb'),export_format='GLB',export_animations=False)
scene.render.filepath=str(WORK/'effects-models.png');bpy.ops.render.render(write_still=True)
print('AYA_EFFECT_MODELS_OK',len(parts),sum(len(p['triangles']) for p in parts),flush=True)
