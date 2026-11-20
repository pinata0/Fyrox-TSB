"""Keep the original smooth shading on the remesh; geometry/motion are unchanged."""
import bpy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from lowpoly_transfer import Surface,transfer_normals
from export_fyrox_fbx import export_fbx
WORK=ROOT/'artifacts/lowpoly-import';STAGE=WORK/'characters'
slugs=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [r['slug'] for r in json.loads((STAGE/'manifest.json').read_text())]
for slug in slugs:
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    body=bpy.data.objects[slug+'_SkinnedMesh']
    with bpy.data.libraries.load(str(WORK/'before'/f'{slug}.blend'),link=False) as (_,loaded):loaded.objects=[slug+'_SkinnedMesh']
    old=loaded.objects[0];reverse=transfer_normals(old,body,Surface(old))
    bpy.data.objects.remove(old,do_unlink=True);bpy.ops.outliner.orphans_purge(do_recursive=True)
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'),relative_remap=False)
    export_fbx(body,bpy.data.objects['TSB_Rig'],STAGE/f'{slug}.fbx',texture_path_mode='STRIP')
    print('LOWPOLY_SMOOTH_NORMALS_OK',slug,'thin-surface guards',reverse,flush=True)
