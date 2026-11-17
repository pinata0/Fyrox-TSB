"""Lower sampled upper lashes with an intermediate blink atlas; retain material drivers."""
import bpy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
import import_meshy_characters as painter
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';painter.STAGE=ASSETS
SLUGS='vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
for slug in (sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS):
    bpy.ops.wm.open_mainfile(filepath=str(ASSETS/f'{slug}.blend'))
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
    face=json.loads((ASSETS/f'{slug}-rig.json').read_text())['face']
    painter.paint_blink_textures(body,face,slug,build_nodes=False,natural=True)
    # The runtime atlas has a distinct halfway pose, rather than fading two eye drawings.
    bpy.context.scene['natural_blink']='sampled upper lashes: open / halfway / closed; browDown follows blink'
    extras=[o for o in bpy.context.scene.objects if o.type=='MESH' and o!=body and o.parent==rig]
    resolve_character_textures([body,*extras])
    for image in bpy.data.images:
        filename=Path(bpy.path.abspath(image.filepath)).name
        if (ASSETS/filename).is_file():image.filepath='//'+filename
    bpy.context.scene.frame_set(0)
    for key in body.data.shape_keys.key_blocks:key.value=0
    bpy.context.preferences.filepaths.save_version=0;bpy.data.use_autopack=False
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/f'{slug}.blend'),relative_remap=False)
    export_fbx(body,rig,ASSETS/f'{slug}.fbx',extras)
    print('NATURAL_BLINK_OK',slug,flush=True)
