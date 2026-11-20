"""Straighten the existing dolphin gun into a swimming whale without changing its UVs."""
import bpy, math, sys
from pathlib import Path
from mathutils import Vector, Matrix
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import resolve_character_textures
assets=ROOT/'assets/characters'
bpy.ops.wm.open_mainfile(filepath=str(assets/'naia-weapon.blend'))
mesh=bpy.data.objects['naia_weapon']
resolve_character_textures((mesh,))
if mesh.data.shape_keys is None:mesh.shape_key_add(name='Basis')
old=mesh.data.shape_keys.key_blocks.get('swimmingWhale')
if old:mesh.shape_key_remove(old)
shape=mesh.shape_key_add(name='swimmingWhale')
for point,base in zip(shape.data,mesh.data.shape_keys.key_blocks[0].data):
    p=base.co.copy()
    # Uncurl the hanging tail into the horizontal swim axis; retain the muzzle,
    # eyes and hat, and broaden the same body into the whale held by both hands.
    t=max(0,min(1,(.12-p.z)/.42));t=t*t*(3-2*t)
    pivot=Vector((0,.015,.12))
    p=pivot+Matrix.Rotation(math.radians(78)*t,3,'X')@(p-pivot)
    p.x*=1.28
    point.co=p
shape.value=0
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(assets/'naia-weapon.blend'),relative_remap=False)
bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True)
bpy.context.view_layer.objects.active=mesh
bpy.ops.export_scene.fbx(filepath=str(assets/'naia-weapon.fbx'),use_selection=True,object_types={'MESH'},axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
print('NAIA_WHALE_MORPH_OK',len(shape.data),flush=True)
