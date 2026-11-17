"""Update existing rigs with 15 individual Select entrances and render roster portraits."""
import bpy, json, sys, traceback
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
import build_characters as author
from export_fyrox_fbx import export_fbx

def main():
    records=json.loads((ROOT/'assets/characters/manifest.json').read_text(encoding='utf-8'))
    for item in records:
        if item.get('source') == 'Tripo (user-provided)':
            # Native bind axes need the dedicated importer; its Select clip and portrait already exist.
            print('SELECT_ASSET_RETAINED_NATIVE_RIG',item['slug'],flush=True)
            continue
        bpy.ops.wm.open_mainfile(filepath=str(ROOT/item['blend']))
        scene=bpy.context.scene; rig=bpy.data.objects['TSB_Rig']; author.RIG=rig
        rig.animation_data_clear()
        for action in list(bpy.data.actions):bpy.data.actions.remove(action)
        author.animate(item['id'])
        scene.frame_start=0;scene.frame_end=528;scene.frame_set(0)
        scene['clips']=json.dumps(author.CLIPS)
        scene['selection_gesture']=author.SELECT_GESTURES[item['id']]
        bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/item['blend']))
        export_fbx(bpy.data.objects[item['slug']+'_SkinnedMesh'],rig,ROOT/item['model'])
        item['clips']=author.CLIPS
        item['selection_gesture']=author.SELECT_GESTURES[item['id']]
        item['portrait']=f"assets/characters/{item['slug']}-portrait.png"
        camera=scene.camera; camera.location=(0,-5,1.55)
        camera.rotation_euler=(Vector((0,0,1.55))-camera.location).to_track_quat('-Z','Y').to_euler()
        camera.data.type='ORTHO';camera.data.ortho_scale=1.30
        scene.render.resolution_x=256;scene.render.resolution_y=256;scene.cycles.samples=10
        scene.render.filepath=str(ROOT/item['portrait'])
        bpy.ops.render.render(write_still=True)
        print('SELECT_ASSET_OK',item['slug'],flush=True)
    (ROOT/'assets/characters/manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2),encoding='utf-8')
    (ROOT/'artifacts/selection-assets-complete.json').write_text(json.dumps({'characters':15,'clips_per_character':11,'frames':[456,528],'portraits':15}),encoding='utf-8')

if __name__=='__main__':
    try:main()
    except Exception:
        (ROOT/'artifacts/selection-assets-error.txt').write_text(traceback.format_exc(),encoding='utf-8')
        raise
