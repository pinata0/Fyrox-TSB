import bpy
import sys
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from import_tripo_butter import components, bounds
specs=json.loads((ROOT/'scripts/tripo_roster_sources.json').read_text())
report={}
for slug,spec in specs.items():
    report[slug]={}
    for kind in spec['parts']:
        bpy.ops.wm.read_factory_settings(use_empty=True)
        path=ROOT/'assets/character-sources/tripo'/slug/kind/f'{slug}-{kind}.fbx'
        bpy.ops.import_scene.fbx(filepath=str(path))
        obj=next(o for o in bpy.context.scene.objects if o.type=='MESH')
        world=obj.matrix_world.copy();obj.parent=None;obj.matrix_world=world
        bpy.ops.object.select_all(action='DESELECT');obj.select_set(True);bpy.context.view_layer.objects.active=obj
        bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
        report[slug][kind]=[{'vertices':len(c),'bounds':bounds(obj,c)} for c in components(obj.data)]
(ROOT/'artifacts/tripo-roster-import/components.json').write_text(json.dumps(report,indent=2))
