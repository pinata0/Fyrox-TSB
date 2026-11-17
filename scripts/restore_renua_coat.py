"""Restore coat faces mistakenly included in the old arm-removal flood fill."""
import bpy,json,sys,shutil
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
from arm_weight_isolation import isolate_refined_arm_weights
ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/cheek-isolation'
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'artifacts/hand-mesh-polish/before/renua.blend'))
source=bpy.data.objects['renua_SkinnedMesh'];points=np.array([v.co[:] for v in source.data.vertices])
weights=[{source.vertex_groups[g.group].name:g.weight for g in v.groups} for v in source.data.vertices]
polys=[tuple(p.vertices) for p in source.data.polygons]
uvs=[[source.data.uv_layers.active.data[i].uv[:] for i in p.loop_indices] for p in source.data.polygons]
def face_key(ids):return tuple(sorted(ids))
bpy.ops.wm.open_mainfile(filepath=str(OUT/'before/renua.blend'))
polished=bpy.data.objects['renua_SkinnedMesh']
assert np.array_equal(points,np.array([v.co[:] for v in polished.data.vertices]))
retained={face_key(p.vertices) for p in polished.data.polygons}
hand=np.zeros(len(points),dtype=bool)
profiles=json.loads((ASSETS/'renua-hands.json').read_text())
for profile in profiles:
    names={profile['hand']}|{control['name'] for control in profile['controls']}
    influence=np.array([sum(w for n,w in row.items() if n in names) for row in weights])
    along=(points-np.array(profile['wrist']))@np.array(profile['forward'])
    hand|=(influence>.45)&(along>-.035)
selected=[i for i,poly in enumerate(polys) if face_key(poly) not in retained and not any(hand[list(poly)])]
assert len(selected)>500,len(selected)
used=sorted({i for j in selected for i in polys[j]});index={old:new for new,old in enumerate(used)}
if not (OUT/'before/renua-before-coat.blend').exists():shutil.copy2(ASSETS/'renua.blend',OUT/'before/renua-before-coat.blend')
bpy.ops.wm.open_mainfile(filepath=str(ASSETS/'renua.blend'))
body=bpy.data.objects['renua_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
old=bpy.data.objects.get('renua_RestoredCoat')
if old:bpy.data.objects.remove(old,do_unlink=True)
mesh=bpy.data.meshes.new('renua_OriginalCoat');mesh.from_pydata(points[used].tolist(),[],[[index[i] for i in polys[j]] for j in selected]);mesh.update()
uv=mesh.uv_layers.new(name='UVMap')
for poly,j in zip(mesh.polygons,selected):
    poly.use_smooth=True
    for loop,coord in zip(poly.loop_indices,uvs[j]):uv.data[loop].uv=coord
coat=bpy.data.objects.new('renua_RestoredCoat',mesh);bpy.context.scene.collection.objects.link(coat);coat.parent=rig;coat.matrix_basis=body.matrix_basis.copy()
groups={name:coat.vertex_groups.new(name=name) for name in sorted({name for i in used for name in weights[i]})}
for new,old in enumerate(used):
    for name,weight in weights[old].items():groups[name].add([new],weight,'REPLACE')
coat.modifiers.new('Original coat skin','ARMATURE').object=rig
isolate_refined_arm_weights(body, rig)
material=bpy.data.materials.new('renua_RestoredCoatMaterial');material.use_nodes=True
bsdf=material.node_tree.nodes['Principled BSDF'];bsdf.inputs['Roughness'].default_value=.82
texture=material.node_tree.nodes.new('ShaderNodeTexImage');texture.image=bpy.data.images.load(str(ASSETS/'renua-meshy-basecolor.png'),check_existing=True)
material.node_tree.links.new(texture.outputs['Color'],bsdf.inputs['Base Color']);mesh.materials.append(material)
for side in ['L','R']:bpy.data.objects[f'renua_RefinedSleeve.{side}'].hide_render=True
for key in body.data.shape_keys.key_blocks:key.value=0
bpy.context.scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0
extras=[o for o in bpy.context.scene.objects if o.type=='MESH' and o!=body and o.parent==rig]
resolve_character_textures([body,*extras])
for image in bpy.data.images:
    filename=Path(bpy.path.abspath(image.filepath)).name
    if (ASSETS/filename).is_file():image.filepath='//'+filename
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'renua.blend'),relative_remap=False)
export_fbx(body,rig,ASSETS/'renua.fbx',extras)
report=dict(restored_faces=len(selected),restored_vertices=len(used),old_hand_vertices_excluded=int(sum(hand)),face_vertices=len(body.data.vertices),facial_controls=len(body.data.shape_keys.key_blocks)-1,new_hand_controls_preserved=True)
(OUT/'renua-coat-restoration.json').write_text(json.dumps(report,indent=2))
record=json.loads((ASSETS/'renua-rig.json').read_text());record['restored_coat']=report;(ASSETS/'renua-rig.json').write_text(json.dumps(record,ensure_ascii=False,indent=2))
print('RENUA_COAT_RESTORED',json.dumps(report),flush=True)
