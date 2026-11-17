"""Preserve sparse FBX morphs before Fyrox 1.0.1 deduplicates its vertex buffer."""
import json, sys
from pathlib import Path
from io_scene_fbx.parse_fbx import parse

ROOT=Path(__file__).resolve().parents[1]
DEST=Path(sys.argv[sys.argv.index('--')+1]) if '--' in sys.argv else ROOT/'assets/characters'
SELECTED=set(sys.argv[sys.argv.index('--')+2:]) if '--' in sys.argv else set()
for path in sorted(DEST.glob('*.fbx')):
    if SELECTED and path.stem not in SELECTED:continue
    root,_=parse(str(path))
    objects=next(e for e in root.elems if e.id==b'Objects')
    connections=next(e for e in root.elems if e.id==b'Connections')
    by_id={e.props[0]:e for e in objects.elems}
    parents={e.props[1]:e.props[2] for e in connections.elems if e.props[0]==b'OO'}
    grouped={}
    for shape in objects.elems:
        if shape.id!=b'Geometry' or shape.props[2]!=b'Shape':continue
        channel=by_id[parents[shape.props[0]]]
        geometry=by_id[parents[parents[channel.props[0]]]]
        parts={e.id:e.props[0] for e in shape.elems if e.props}
        name=channel.props[1].split(b'\x00')[0].decode().split('.')[-1]
        grouped.setdefault(geometry.props[0],[]).append(dict(name=name,indices=list(parts[b'Indexes']),offsets=list(parts[b'Vertices'])))
    if not grouped:continue
    assert len(grouped)==1,path
    geometry_id,shapes=next(iter(grouped.items()))
    vertices=list(next(e for e in by_id[geometry_id].elems if e.id==b'Vertices').props[0])
    # Closed-eye normals must replace the neutral eye-rim shading, including
    # custom normals retained by facial refinement. Store absolute targets so
    # each imported UV/hard-normal duplicate can compute its own delta.
    blend=path.with_suffix('.blend')
    if blend.is_file():
        import bpy
        import numpy as np
        bpy.ops.wm.open_mainfile(filepath=str(blend))
        body=bpy.data.objects.get(path.stem+'_SkinnedMesh')
        if body and body.data.attributes.get('blink_normal_L'):
            mesh=body.data
            points=np.array([v.co[:] for v in mesh.vertices])
            assert np.allclose(points,np.array(vertices).reshape(-1,3),atol=1e-6),path
            for shape in shapes:
                if not shape['name'].startswith('eyeBlink_'):continue
                side=shape['name'][-1]
                normal=mesh.attributes.get('blink_normal_'+side)
                weight=mesh.attributes.get('blink_weight_'+side)
                if normal is None or weight is None:continue
                selected=[i for i,w in enumerate(weight.data) if w.value>1e-5]
                shape['normal_indices']=selected
                shape['normal_targets']=[[*normal.data[i].vector[:],weight.data[i].value] for i in selected]
    data=dict(vertices=vertices,shapes=shapes)
    path.with_suffix('.morphs.json').write_text(json.dumps(data,separators=(',',':')))
    print('MESHY_MORPH_SOURCE_OK',path.stem,len(shapes),flush=True)
