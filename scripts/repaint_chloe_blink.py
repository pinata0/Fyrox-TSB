"""Refresh Chloe's closed lids while retaining the refined mesh and arm meshes."""
import bpy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
import import_meshy_characters as painter
from export_fyrox_fbx import export_fbx
ASSETS=ROOT/'assets/characters';painter.STAGE=ASSETS
bpy.ops.wm.open_mainfile(filepath=str(ASSETS/'chloe.blend'))
body=bpy.data.objects['chloe_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig'];mat=body.data.materials[0];nodes=mat.node_tree.nodes
base=next(n for n in nodes if n.type=='TEX_IMAGE' and n.image and n.image.name==mat['meshy_base_texture'])
for node in list(nodes):
    if node!=base and node.type in ['TEX_IMAGE','MIX_RGB']:nodes.remove(node)
for image in list(bpy.data.images):
    if image.name.startswith('chloe-blink-'):bpy.data.images.remove(image)
mat.node_tree.links.new(base.outputs['Color'],nodes.get('Principled BSDF').inputs['Base Color'])
painter.paint_blink_textures(body,json.loads((ASSETS/'chloe-rig.json').read_text())['face'],'chloe')
for image in bpy.data.images:
    if image.source=='FILE' and (ASSETS/Path(image.filepath).name).exists():image.filepath='//'+Path(image.filepath).name
for key in body.data.shape_keys.key_blocks:key.value=0
bpy.context.scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'chloe.blend'),relative_remap=False)
export_fbx(body,rig,ASSETS/'chloe.fbx',[o for o in bpy.data.objects if o.type=='MESH' and o!=body and o.parent==rig])
print('CHLOE_BLINK_REFRESH_OK',flush=True)
