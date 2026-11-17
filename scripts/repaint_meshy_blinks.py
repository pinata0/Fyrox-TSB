"""Refresh UV-atlas eyelid textures without rebuilding validated skin or motion."""
import bpy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from import_meshy_characters import STAGE,OUT,SLUGS,paint_blink_textures,setup_scene
from export_fyrox_fbx import export_fbx
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS
for slug in selected:
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    mesh=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig'];mat=mesh.data.materials[0];mat.node_tree.animation_data_clear()
    for n in list(mat.node_tree.nodes):
        if n.type not in ['OUTPUT_MATERIAL','BSDF_PRINCIPLED'] and not (n.type=='TEX_IMAGE' and n.image and 'meshy-basecolor' in n.image.filepath):mat.node_tree.nodes.remove(n)
    for image in list(bpy.data.images):
        if image.name.startswith(f'{slug}-blink-'):bpy.data.images.remove(image)
    record=json.loads((STAGE/f'{slug}-rig.json').read_text());paint_blink_textures(mesh,record['face'],slug)
    bpy.context.scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'));export_fbx(mesh,rig,STAGE/f'{slug}.fbx',[o for o in bpy.context.scene.objects if o.type=='MESH' and o!=mesh])
    for k in mesh.data.shape_keys.key_blocks:
        if k.name.startswith('eyeBlink'):k.value=1
    setup_scene(mesh,OUT/f'{slug}-expression.png',face=True)
    print('MESHY_BLINK_ATLAS_OK',slug,flush=True)
