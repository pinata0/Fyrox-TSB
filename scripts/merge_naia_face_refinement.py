"""Keep Naia's independently edited braids/attack while replacing interim face tessellation."""
import bpy,json,sys,shutil
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from refine_face_triangles import refine
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/cheek-isolation';path=ASSETS/'naia.blend'
if not (OUT/'before/naia-concurrent.blend').exists():shutil.copy2(path,OUT/'before/naia-concurrent.blend')
bpy.ops.wm.open_mainfile(filepath=str(path));body=bpy.data.objects['naia_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
latest=body.data;coords=np.array([v.co[:] for v in latest.vertices]);group_names=[g.name for g in body.vertex_groups]
weights=[[(group_names[g.group],g.weight) for g in v.groups] for v in latest.vertices]
materials=list(latest.materials);old_keys=latest.shape_keys
old_action=old_keys.animation_data.action if old_keys.animation_data else None
old_slot=old_keys.animation_data.action_slot if old_action else None
with bpy.data.libraries.load(str(OUT/'before/naia.blend'),link=False) as (available,loaded):loaded.objects=['naia_SkinnedMesh']
original=loaded.objects[0].data;original_coords=np.array([v.co[:] for v in original.vertices])
assert np.array_equal(original_coords,coords[:len(original_coords)])
body.data=original
for name in group_names:
    if name not in body.vertex_groups:body.vertex_groups.new(name=name)
for group in body.vertex_groups:group.remove(list(range(len(original.vertices))))
for i,row in enumerate(weights[:len(original.vertices)]):
    for name,weight in row:body.vertex_groups[name].add([i],weight,'REPLACE')
original.materials.clear()
for material in materials:
    original.materials.append(material)
    if material.node_tree.animation_data:
        for curve in material.node_tree.animation_data.drivers:
            for variable in curve.driver.variables:
                for target in variable.targets:
                    if target.id==old_keys:target.id=original.shape_keys
record=json.loads((ASSETS/'naia-rig.json').read_text());mesh=refine(body,record['face'])
if old_action:
    mesh.shape_keys.animation_data_create();mesh.shape_keys.animation_data.action=old_action;mesh.shape_keys.animation_data.action_slot=old_slot
assert len(rig.data.bones)==58
assert all(f'Braid.{side}.{i:02}' in body.vertex_groups for side in ['L','R'] for i in [1,2,3])
record.update(vertices=len(mesh.vertices),triangles=len(mesh.polygons),bones=len(rig.data.bones))
record['cheek_refinement']=dict(original_vertices=len(original_coords),vertices=len(mesh.vertices),maximum_edge_m=.012,interpolated_face_channels=23,braids_preserved=True)
extras=[o for o in bpy.context.scene.objects if o.type=='MESH' and o!=body and o.parent==rig]
resolve_character_textures([body,*extras]);bpy.context.scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(path),relative_remap=False);export_fbx(body,rig,path.with_suffix('.fbx'),extras)
(ASSETS/'naia-rig.json').write_text(json.dumps(record,ensure_ascii=False,indent=2))
print('NAIA_REFINEMENT_AND_BRAIDS_MERGED',len(mesh.vertices),len(rig.data.bones),len(extras),flush=True)
