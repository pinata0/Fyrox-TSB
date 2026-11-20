"""Check that the white-lash repair preserves the character and its animations."""
import bpy,hashlib,json,sys
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'));sys.dont_write_bytecode=True
from id_eyelids import lash_weight
OUT=ROOT/'artifacts/id-blink-fix'
published='--published' in sys.argv
DEST=ROOT/'assets/characters' if published else OUT/'candidate'

def digest(a):return hashlib.sha256(np.asarray(a,dtype=np.float32).tobytes()).hexdigest()
def snapshot(folder):
    bpy.ops.wm.open_mainfile(filepath=str(folder/'id.blend'))
    result={}
    for obj in bpy.context.scene.objects:
        if obj.type!='MESH':continue
        m=obj.data
        result[obj.name]=dict(points=digest([v.co[:] for v in m.vertices]),
            topology=digest([l.vertex_index for l in m.loops]),
            uv=digest([u.uv[:] for u in m.uv_layers.active.data]),
            weights=digest([(v.index,g.group,g.weight) for v in m.vertices for g in v.groups]),
            groups=[g.name for g in obj.vertex_groups],
            shapes={k.name:digest([v.co[:] for v in k.data]) for k in m.shape_keys.key_blocks
                    if not k.name.startswith('eyeBlink_')} if m.shape_keys else {})
    rig=bpy.data.objects['TSB_Rig']
    result['bones']=[(b.name,b.parent.name if b.parent else None,list(b.head_local),list(b.tail_local)) for b in rig.data.bones]
    result['clips']=bpy.context.scene.get('clips')
    animation=[]
    for action in bpy.data.actions:
        for layer in action.layers:
            for strip in layer.strips:
                for bag in strip.channelbags:
                    for curve in bag.fcurves:
                        animation.append((action.name,curve.data_path,curve.array_index,
                            [(tuple(k.co),tuple(k.handle_left),tuple(k.handle_right),k.interpolation) for k in curve.keyframe_points]))
    result['animation']=animation
    return result

before=snapshot(OUT/'before');after=snapshot(DEST)
assert before==after,'Neutral geometry, UVs, weights, other expressions or animation changed'
mesh=bpy.data.objects['id_SkinnedMesh'].data
basis=np.array([v.co[:] for v in mesh.vertices])
face=json.loads((DEST/'id-rig.json').read_text())['face']
stats={}
for side in ['L','R']:
    weight=lash_weight(basis,face,side)
    core=weight>.99
    blink=np.array([v.co[:] for v in mesh.shape_keys.key_blocks['eyeBlink_'+side].data])-basis
    brow=np.array([v.co[:] for v in mesh.shape_keys.key_blocks['browDown_'+side].data])-basis
    closed=basis+blink+.75*brow
    assert core.sum()>200
    assert np.max(np.abs(blink[core,1]))<.001
    assert np.all((closed-basis)[core,2]<-.045)
    assert np.all(np.isfinite(closed))
    stats[side]=dict(core_vertices=int(core.sum()),
                    median_drop=float(np.median((basis-closed)[core,2])))
morph=json.loads((DEST/'id.morphs.json').read_text())
old=json.loads((OUT/'before/id.morphs.json').read_text())
assert morph['vertices']==old['vertices']
assert len(morph['shapes'])==len(old['shapes'])==23
for shape in old['shapes']:
    current=next(s for s in morph['shapes'] if s['name']==shape['name'])
    if not shape['name'].startswith('eyeBlink_'):assert current==shape,shape['name']
report=dict(neutral_mesh_uv_weights_other_shapes_and_animations_preserved=True,facial_channels=23,lashes=stats)
report['candidate_sha256']={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in DEST.glob('id*') if p.is_file() and p.name in {q.name for q in (OUT/'candidate').iterdir()}}
(OUT/('published-verification.json' if published else 'verification.json')).write_text(json.dumps(report,indent=2))
print('ID_BLINK_VERIFIED',report,flush=True)
