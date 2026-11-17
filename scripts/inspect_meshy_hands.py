import bpy,json,sys
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from import_meshy_characters import MAPS,native,SLUGS
OUT=ROOT/'artifacts/meshy-hand-fix';OUT.mkdir(parents=True,exist_ok=True)
reports=[]
for slug in SLUGS:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(ROOT/f'assets/character-sources/meshy/{slug}/{slug}-source.glb'))
    arm=next(o for o in bpy.data.objects if o.type=='ARMATURE')
    source={}
    for side,idx in [('L',MAPS[slug][2][-1]),('R',MAPS[slug][3][-1])]:
        bone=arm.data.bones[native(idx)]
        source[side]=[dict(name=b.name,parent=b.parent.name if b.parent else None,head=list(arm.matrix_world@b.head_local),tail=list(arm.matrix_world@b.tail_local)) for b in [bone,*bone.children_recursive]]
    bpy.ops.wm.open_mainfile(filepath=str(ROOT/f'assets/characters/{slug}.blend'))
    arm=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects[f'{slug}_SkinnedMesh'];bpy.context.scene.frame_set(0)
    current={}
    for side in ['L','R']:
        group=mesh.vertex_groups[f'Hand.{side}'].index
        points=[v.co[:] for v in mesh.data.vertices if any(g.group==group and g.weight>.5 for g in v.groups)]
        current[side]=dict(wrist=list(arm.data.bones[f'Hand.{side}'].head_local),elbow=list(arm.data.bones[f'Forearm.{side}'].head_local),points=points)
    reports.append(dict(slug=slug,source=source,current=current))
    print('HAND_INSPECT',slug,[(s,len(source[s]),len(current[s]['points'])) for s in ['L','R']],flush=True)
(OUT/'hand-inspection.json').write_text(json.dumps(reports))
