"""Separate the existing textured clover from its fixed staff for runtime animation."""
import bpy,bmesh,shutil,json,hashlib
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];A=ROOT/'assets/characters';B=ROOT/'artifacts/ui-skills/before';B.mkdir(parents=True,exist_ok=True)
for n in ['ui-weapon.blend','ui-weapon.fbx']:
 if not (B/n).exists():shutil.copy2(A/n,B/n)
bpy.ops.wm.open_mainfile(filepath=str(B/'ui-weapon.blend'))
body=bpy.data.objects['ui_weapon'];bpy.context.view_layer.objects.active=body;body.select_set(True)
for image in bpy.data.images:
 if image.source=='FILE':
  p=A/Path(bpy.path.abspath(image.filepath)).name
  if p.is_file():image.filepath=str(p)
# Keep original UVs and material slots on both pieces. No remeshing or repainting.
clover=body.copy();clover.data=body.data.copy();bpy.context.collection.objects.link(clover);clover.name='Ui_Clover'
# A narrow fixed axle extends the stem behind the revolving leaf. Sample its
# green UV from the original lower stem; do not stretch any of the yellow bow.
stem_face=min(body.data.polygons,key=lambda f:((body.matrix_world@f.center)-Vector((0,-.02,-.05))).length)
uv=sum((body.data.uv_layers.active.data[i].uv.copy() for i in stem_face.loop_indices),Vector((0,0)))/len(stem_face.loop_indices)
bpy.ops.mesh.primitive_cylinder_add(vertices=16,radius=.009,depth=.20,location=(0,.012,.27))
axle=bpy.context.object;axle.name='Ui_Clover_Axle';axle.data.materials.append(body.data.materials[stem_face.material_index])
for loop in axle.data.uv_layers.active.data:loop.uv=uv
for obj,top in [(body,False),(clover,True)]:
 bm=bmesh.new();bm.from_mesh(obj.data)
 def is_leaf(f):
  p=sum((obj.matrix_world@v.co for v in f.verts),Vector((0,0,0)))/len(f.verts)
  return p.z>.219 or (p.z>.195 and p.x>.022)
 remove=[f for f in bm.faces if is_leaf(f)!=top]
 bmesh.ops.delete(bm,geom=remove,context='FACES');bm.to_mesh(obj.data);bm.free()
 # Put the leaf hub at its true center, leaving mesh conversion independent.
 if top:
  center=Vector((0,0,.365))
  for v in obj.data.vertices:v.co-=obj.matrix_world.inverted().to_3x3()@center
  obj.location+=center
for obj in (body,clover,axle):
 for p in obj.data.polygons:p.use_smooth=True
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(A/'ui-weapon.blend'),relative_remap=False)
bpy.ops.object.select_all(action='DESELECT')
for obj in (body,clover,axle):obj.select_set(True)
bpy.ops.export_scene.fbx(filepath=str(A/'ui-weapon.fbx'),use_selection=True,object_types={'MESH'},apply_unit_scale=True,axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
(ROOT/'artifacts/ui-skills/clover-split.json').write_text(json.dumps({'pivot':[0,0,.365],'triangles':[len(o.data.polygons) for o in (body,clover,axle)]}))
print('UI_CLOVER_SPLIT_OK',flush=True)
