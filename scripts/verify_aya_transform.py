"""Asset integrity checks for the modified existing parasol and solid blade meshes."""
import bpy,bmesh,hashlib,json
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'artifacts/aya-transform'
def topology(mesh):
 return [tuple(p.vertices) for p in mesh.data.polygons]
def uvs(mesh):
 return [[tuple(d.uv) for d in layer.data] for layer in mesh.data.uv_layers]
bpy.ops.wm.open_mainfile(filepath=str(OUT/'before/assets/characters/aya-weapon.blend'))
old=bpy.data.objects['aya_weapon'];faces=topology(old);coords=uvs(old)
bpy.ops.wm.open_mainfile(filepath=str(ROOT/'assets/characters/aya-weapon.blend'))
mesh=bpy.data.objects['aya_weapon'];assert topology(mesh)==faces;assert uvs(mesh)==coords
assert len(mesh.data.vertices)==5965
keys=mesh.data.shape_keys.key_blocks
assert [k.name for k in keys]==['Basis','parasolFold','parasolSpread']
assert all(abs(k.value)<.001 for k in list(keys)[1:])
p=np.array([v.co[:] for v in keys['Basis'].data]);q=np.array([v.co[:] for v in keys['parasolFold'].data])
assert np.isfinite(q).all()
assert np.ptp(q[:,:2],axis=0).max()<np.ptp(p[:,:2],axis=0).max()*.25
for n in range(1,4):
 o=bpy.data.objects[f'Aya_Blade_{n}'];bm=bmesh.new();bm.from_mesh(o.data)
 assert all(e.is_manifold for e in bm.edges),o.name
 assert bm.calc_volume(signed=True)>0,o.name
 bm.free()
scene=bpy.context.scene
for frame in [0,45,90,178]:
 scene.frame_set(frame)
 assert keys['parasolFold'].value<.001
 assert all(max(bpy.data.objects[f'Aya_Blade_{n}'].scale)<.001 for n in range(1,4))
for frame,blade in [(18,1),(120,2),(150,3)]:
 scene.frame_set(frame)
 assert keys['parasolFold'].value>.99
 assert max(bpy.data.objects[f'Aya_Blade_{blade}'].scale)>.99
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=str(ROOT/'assets/characters/aya-weapon.fbx'))
mesh=bpy.data.objects['aya_weapon'];assert len(mesh.data.shape_keys.key_blocks)==3
assert all(f'Aya_Blade_{n}' in bpy.data.objects for n in range(1,4))
for n in range(1,4):assert all(len(p.vertices)==3 for p in bpy.data.objects[f'Aya_Blade_{n}'].data.polygons)
paths=['aya-weapon.blend','aya-weapon.fbx','aya-weapon.morphs.json','aya.blend','aya.fbx']
report={'original_topology_and_uvs_preserved':True,'fold_reduces_canopy_width':True,'blade_meshes_closed_with_outward_normals':True,'fbx_blades_triangulated':True,'both_attacks_reset_at_end':True,'sha256':{n:hashlib.sha256((ROOT/'assets/characters'/n).read_bytes()).hexdigest() for n in paths}}
(OUT/'weapon-verification.json').write_text(json.dumps(report,indent=2))
print('AYA_TRANSFORM_ASSETS_VERIFIED',flush=True)
