"""Keep the wooden fork rigid; move only the leather pouch and rubber bands."""
import bpy, bmesh, json, sys, shutil, runpy, math
import numpy as np
from mathutils import Vector
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/characters'
OUT = ROOT / 'artifacts/butter-skills/slingshot-fix'
OUT.mkdir(parents=True, exist_ok=True)
for suffix in ['blend', 'fbx', 'morphs.json']:
    source = ASSETS / f'butter-weapon.{suffix}'
    backup = OUT / source.name
    if not backup.exists():
        shutil.copy2(source, backup)
bpy.ops.wm.open_mainfile(filepath=str(OUT / 'butter-weapon.blend'))
obj = bpy.data.objects['butter_weapon']
mesh = obj.data
points = np.array([v.co[:] for v in mesh.vertices])
image = next(n.image for m in mesh.materials for n in m.node_tree.nodes if n.type == 'TEX_IMAGE')
image.filepath = str(ASSETS / 'butter-weapon-meshy-basecolor.png')
pixels = np.array(image.pixels[:]).reshape(image.size[1], image.size[0], 4)
colors = np.zeros((len(points), 3)); counts = np.zeros(len(points))
for loop in mesh.loops:
    uv = mesh.uv_layers.active.data[loop.index].uv
    colors[loop.vertex_index] += pixels[int(uv.y*(image.size[1]-1)), int(uv.x*(image.size[0]-1)), :3]
    counts[loop.vertex_index] += 1
colors /= np.maximum(1,counts)[:,None]
neighbors=[set() for _ in points]
for edge in mesh.edges:
    a,b=edge.vertices;neighbors[a].add(b);neighbors[b].add(a)
# Find whole connected yellow-wood regions, excluding tiny painted flecks on
# the pouch. Grow into their dark border vertices, where texture hue is unstable.
yellow=(colors[:,1]>colors[:,2]*1.7)&(colors[:,1]-colors[:,2]>.04)
unseen=set(np.flatnonzero(yellow));wood=set();components=[]
while unseen:
    pending=[unseen.pop()];component=set(pending)
    while pending:
        for j in neighbors[pending.pop()]:
            if j in unseen: unseen.remove(j);component.add(j);pending.append(j)
    components.append(len(component))
    if len(component)>=30:wood.update(component)
for _ in range(2): wood.update(j for i in list(wood) for j in neighbors[i])
# Bound the connected inner pouch/bands geometrically. Texture highlights and
# small painted specks must not leave frozen vertices inside the moving leather.
x, y, z = points.T
def smooth(t):
    t=np.clip(t,0,1);return t*t*(3-2*t)
opening = (z > .12) & (z < .275) & (abs(x) < .090 + np.maximum(0, z-.21)*.45)
opening &= np.array([i not in wood for i in range(len(points))])
weights = np.where(opening & (z<.225),1.0,0.0) # leather pouch moves rigidly
selected=weights>0
assert 100 < np.count_nonzero(weights) < 2500
keys = mesh.shape_keys.key_blocks
pull = keys['bandPull']
lift = keys.get('giantBandLift') or obj.shape_key_add(name='giantBandLift', from_mix=False)
for i, point in enumerate(points):
    pull.data[i].co = point + (0, .32*weights[i], 0)
    lift.data[i].co = point + (0, 0, .085*weights[i])
pull.value = lift.value = 0
assert all(np.array_equal(np.array(pull.data[i].co),points[i]) for i in np.flatnonzero(~selected))
assert all(np.array_equal(np.array(lift.data[i].co),points[i]) for i in np.flatnonzero(~selected))
# The reconstructed source has tiny surface bridges from the pouch's bottom to
# the wooden fork. Cut those fused faces so pulling the pouch cannot drag a web
# of triangles out of the otherwise rigid opening. Keep the upper band anchors.
bridges=[p.index for p in mesh.polygons if np.mean([points[i,2] for i in p.vertices])<.23
         and max(weights[i] for i in p.vertices)>.10 and min(weights[i] for i in p.vertices)==0]
bm=bmesh.new();bm.from_mesh(mesh);bm.faces.ensure_lookup_table()
old_bands=[p.index for p in mesh.polygons if any(opening[i] for i in p.vertices)]
for polygon in mesh.polygons:
    ids=list(polygon.vertices);color=colors[ids].mean(0)
    if .08<abs(points[ids,0].mean())<.13 and points[ids,2].min()<.267 and .21<points[ids,2].mean()<.29 and color[2]>.6*color[1] and color[0]>1.2*color[1]:
        old_bands.append(polygon.index)
bmesh.ops.delete(bm,geom=[bm.faces[i] for i in set(bridges+old_bands)],context='FACES_ONLY')
bm.to_mesh(mesh);bm.free()
assert len(mesh.vertices)==len(points)
# Closed rounded ribbons replace the irregular reconstructed rubber. Separate
# giant keys preserve their cross-sections with the raised pouch.
poses={name:[Vector(p) for p in points] for name in ['Basis','bandPull','giantBandLift','giantBandPull']}
faces=[list(p.vertices) for p in mesh.polygons]
uvs=[[mesh.uv_layers.active.data[i].uv.copy() for i in p.loop_indices] for p in mesh.polygons]
materials=[p.material_index for p in mesh.polygons];old_materials=list(mesh.materials)
rubber=bpy.data.materials.new('Butter elastic ribbon');rubber.use_nodes=True
rubber.diffuse_color=(.40,.12,.09,1)
rubber.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=rubber.diffuse_color
rubber.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.85
# A separate rounded leather cup replaces the fused source pouch. Its rim is
# dark and its solid front has no holes through which the loaded rock can show.
leather=bpy.data.materials.new('Butter leather cup');leather.use_nodes=True
leather.diffuse_color=(.55,.29,.17,1)
leather.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=leather.diffuse_color
leather.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.9
cup_start=len(poses['Basis']);cup=[];count=32
for scale,depth in [(1.0,0.0),(.92,.012),(.90,-.012)]:
    for i in range(count):
        a=i*math.tau/count;px=.074*math.cos(a)*scale
        pz=.044*math.sin(a)*scale-.007*max(0,math.sin(a))*math.exp(-(px/.024)**2)
        cup.append(Vector((px,depth,.18+pz)))
cup += [Vector((0,.015,.18)),Vector((0,-.012,.18))]
for name,offset in [('Basis',(0,0,0)),('bandPull',(0,.32,0)),('giantBandLift',(0,0,.085)),('giantBandPull',(0,.32,0))]:poses[name]+=[v+Vector(offset) for v in cup]
for i in range(count):
    j=(i+1)%count
    for local,mat in [([i,j,count+j,count+i],len(old_materials)),([i,2*count+i,2*count+j,j],len(old_materials)),([count+i,count+j,3*count],len(old_materials)+1),([2*count+j,2*count+i,3*count+1],len(old_materials)+1)]:
        faces.append([cup_start+k for k in local]);uvs.append([(0,0)]*len(local));materials.append(mat)
for side in [-1,1]:
    anchor=Vector((side*.112,0,.285));pouch=Vector((side*.060,0,.180))
    def ribbon(draw,raise_pouch):
        end=pouch+Vector((0,.32*draw,.085*raise_pouch));axis=(end-anchor).normalized()
        width=(Vector((1,0,0))-axis*axis.x).normalized();depth=axis.cross(width).normalized()
        return [anchor.lerp(end,j/12)+width*(.011*math.cos(k*math.tau/8))+depth*(.0035*math.sin(k*math.tau/8)) for j in range(13) for k in range(8)]
    neutral=ribbon(0,0);normal=ribbon(1,0);raised=ribbon(0,1);giant=ribbon(1,1)
    start=len(poses['Basis'])
    poses['Basis']+=neutral;poses['bandPull']+=normal;poses['giantBandLift']+=raised
    poses['giantBandPull'] += [base+draw-rest for base,draw,rest in zip(neutral,giant,raised)]
    for j in range(12):
        for k in range(8):
            faces.append([start+j*8+k,start+j*8+(k+1)%8,start+(j+1)*8+(k+1)%8,start+(j+1)*8+k]);uvs.append([(0,0)]*4);materials.append(len(old_materials))
    for ring in [0,12]:
        faces.append([start+ring*8+k for k in (range(7,-1,-1) if ring==0 else range(8))]);uvs.append([(0,0)]*8);materials.append(len(old_materials))
rebuilt=bpy.data.meshes.new('Butter rigid frame and separate elastic')
rebuilt.from_pydata(poses['Basis'],[],faces);rebuilt.update();obj.data=rebuilt
for material in old_materials+[rubber,leather]:rebuilt.materials.append(material)
uv=rebuilt.uv_layers.new(name='UVMap')
for polygon,coords,material in zip(rebuilt.polygons,uvs,materials):
    polygon.material_index=material;polygon.use_smooth=True
    for loop,coord in zip(polygon.loop_indices,coords):uv.data[loop].uv=coord
for name,vertices in poses.items():
    key=obj.shape_key_add(name=name,from_mix=False)
    for vertex,position in zip(key.data,vertices):vertex.co=position
mesh=rebuilt;triangle_count=sum(len(p.vertices)-2 for p in mesh.polygons);vertex_count=len(mesh.vertices)
bpy.context.preferences.filepaths.save_version = 0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS / 'butter-weapon.blend'), relative_remap=False)
bpy.ops.object.select_all(action='DESELECT'); obj.select_set(True); bpy.context.view_layer.objects.active=obj
bpy.ops.export_scene.fbx(filepath=str(ASSETS / 'butter-weapon.fbx'), use_selection=True, object_types={'MESH'}, axis_forward='-Z', axis_up='Y', bake_anim=False, path_mode='AUTO')
sys.argv = ['export_meshy_morphs.py', '--', str(ASSETS), 'butter-weapon']
runpy.run_path(str(ROOT / 'scripts/export_meshy_morphs.py'), run_name='__main__')
prop_path=ASSETS/'butter-weapon-prop.json'
prop=json.loads(prop_path.read_text());prop['vertices']=vertex_count;prop['triangles']=triangle_count
prop_path.write_text(json.dumps(prop,indent=2)+'\n')
(OUT/'verification.json').write_text(json.dumps({'vertices':vertex_count,'new_pouch_vertices':98,'new_band_vertices':208,'original_vertices_kept_rigid':len(points),'removed_fused_faces':len(bridges),'replaced_band_faces':len(set(old_bands)),'pouch_raise':.085}, indent=2))
print('BUTTER_SLINGSHOT_FIXED',vertex_count,'vertices; rigid frame, separate cup and ribbons',flush=True)
