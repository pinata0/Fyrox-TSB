"""Check neutral geometry, UVs, skinning and unrelated morphs are preserved."""
import bpy,hashlib,json,sys
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from blink_surfaces import protected_vertices
OUT=ROOT/'artifacts/blink-surface-fix'

def digest(values,dtype=np.float32):
    return hashlib.sha256(np.asarray(values,dtype=dtype).tobytes()).hexdigest()

def snapshot(path):
    bpy.ops.wm.open_mainfile(filepath=str(path))
    result={}
    for obj in bpy.context.scene.objects:
        if obj.type=='MESH':
            m=obj.data
            result[obj.name]=dict(
                points=digest([v.co[:] for v in m.vertices]),
                loops=digest([l.vertex_index for l in m.loops],np.int32),
                uv=digest([u.uv[:] for u in m.uv_layers.active.data]) if m.uv_layers.active else None,
                groups=[g.name for g in obj.vertex_groups],
                weights=digest([(v.index,g.group,g.weight) for v in m.vertices for g in v.groups]),
                shapes={k.name:digest([v.co[:] for v in k.data]) for k in m.shape_keys.key_blocks
                        if not k.name.startswith('eyeBlink_')} if m.shape_keys else {},
            )
        elif obj.type=='ARMATURE':
            result[obj.name]=dict(bones={b.name:[v for row in b.matrix_local for v in row] for b in obj.data.bones})
    return result

report=json.loads((OUT/'asset-verification.json').read_text()) if '--' in sys.argv and (OUT/'asset-verification.json').exists() else {}
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [p.stem for p in (OUT/'candidate').glob('*.blend')]
for slug in selected:
    before=snapshot(OUT/'before'/f'{slug}.blend')
    after=snapshot(OUT/'candidate'/f'{slug}.blend')
    if before!=after:
        differences={name:[key for key in before.get(name,{}) if before[name][key]!=after.get(name,{}).get(key)]
                     for name in before.keys()|after.keys() if before.get(name)!=after.get(name)}
        raise AssertionError((slug,'neutral geometry, UVs, skinning or unrelated morph changed',differences))
    mesh=bpy.data.objects[f'{slug}_SkinnedMesh'].data
    basis=np.array([v.co[:] for v in mesh.vertices])
    channels={}
    for side in (['L'] if slug=='aya' else ['L','R']):
        weight=np.array([v.value for v in mesh.attributes[f'blink_weight_{side}'].data])
        normal=np.array([v.vector[:] for v in mesh.attributes[f'blink_normal_{side}'].data])
        target=np.array([v.co[:] for v in mesh.shape_keys.key_blocks[f'eyeBlink_{side}'].data])
        assert np.isfinite(target).all() and np.isfinite(normal).all()
        assert ((weight>=0)&(weight<=1)).all()
        assert np.max(np.abs(np.linalg.norm(normal,axis=1)-1))<1e-5
        assert np.array_equal(basis[:,[0,2]],target[:,[0,2]])
        assert np.array_equal(basis[weight==0],target[weight==0])
        guarded=protected_vertices(bpy.data.objects[f'{slug}_SkinnedMesh'])
        assert not weight[guarded].any(),(slug,'glasses/hair clip moved')
        assert np.count_nonzero(weight>.99)>30
        assert np.max(np.abs(target-basis))<.03
        channels[side]=dict(vertices=int(np.count_nonzero(weight>0)),core=int(np.count_nonzero(weight>.99)))
    morph=json.loads((OUT/'candidate'/f'{slug}.morphs.json').read_text())
    for shape in morph['shapes']:
        if shape['name'] in ['eyeBlink_'+side for side in channels]:
            assert len(shape['normal_indices'])==len(shape['normal_targets'])>30
    report[slug]=dict(unchanged_neutral_uv_weights_bones_other_shapes=True,channels=channels)
    print('BLINK_ASSET_VERIFIED',slug,flush=True)
(OUT/'asset-verification.json').write_text(json.dumps(report,indent=2))
