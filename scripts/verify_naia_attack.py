"""Check independent hair skinning, animation, intact morphs and FBX export."""
import bpy,json,sys,math,struct,zlib
import numpy as np
from pathlib import Path
from mathutils import Quaternion
ROOT=Path(__file__).resolve().parents[1];ASSETS=ROOT/'assets/characters';OUT=ROOT/'artifacts/naia-attack'
bpy.ops.wm.open_mainfile(filepath=str(ASSETS/'naia.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects['naia_SkinnedMesh']
assert len(rig.data.bones)==58
assert len(mesh.data.shape_keys.key_blocks)==24
assert all(abs(sum(g.weight for g in v.groups)-1)<1e-4 and len(v.groups)<=4 for v in mesh.data.vertices)
for side in ['L','R']:
    for i in range(3):
        b=rig.data.bones[f'Braid.{side}.{i+1:02}']
        assert b.parent.name==('Head' if i==0 else f'Braid.{side}.{i:02}')
        if i:assert (b.head_local-b.parent.tail_local).length<1e-6
def points():
    bpy.context.view_layer.update()
    obj=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    return np.array([v.co[:] for v in obj.data.vertices])
scene.frame_set(0);before=points()
groups={s:{mesh.vertex_groups[f'Braid.{s}.{i+1:02}'].index for i in range(3)} for s in ['L','R']}
affected={s:[v.index for v in mesh.data.vertices if any(g.group in groups[s] and g.weight>.5 for g in v.groups)] for s in ['L','R']}
assert all(len(v)>500 for v in affected.values())
independent={}
for side,other in [('L','R'),('R','L')]:
    scene.frame_set(0)
    bone=rig.pose.bones[f'Braid.{side}.01'];q=bone.rotation_quaternion.copy()
    bone.rotation_quaternion=q@Quaternion((1,0,0),.35)
    changed=points();moved=np.linalg.norm(changed-before,axis=1)
    assert max(moved[affected[other]])<1e-6,(side,'opposite braid moves')
    assert max(moved[affected[side]])>.08,(side,'braid does not bend')
    independent[side]=float(max(moved[affected[side]]));bone.rotation_quaternion=q
max_edge=0;edge_ids=np.array([e.vertices[:] for e in mesh.data.edges])
rest=np.array([v.co[:] for v in mesh.data.vertices]);lengths=np.linalg.norm(rest[edge_ids[:,0]]-rest[edge_ids[:,1]],axis=1)
hair_edges=np.any(np.isin(edge_ids,np.array(affected['L']+affected['R'])),axis=1)&(lengths>.004)
for frame in np.linspace(156,180,101):
    scene.frame_set(int(frame),subframe=float(frame%1));p=points()
    assert np.isfinite(p).all()
    ratios=np.linalg.norm(p[edge_ids[:,0]]-p[edge_ids[:,1]],axis=1)[hair_edges]/lengths[hair_edges]
    max_edge=max(max_edge,float(max(ratios)))
assert max_edge<2.0,('overstretched braid',max_edge)
scene.frame_set(156);start=points();scene.frame_set(180);end=points()
assert np.max(np.abs(start-end))<.002,('attack recovery seam',np.max(np.abs(start-end)))
# Parse the delivered FBX and compare its sparse facial source, not just Blender state.
from io_scene_fbx.parse_fbx import parse
fbx,_=parse(str(ASSETS/'naia.fbx'));objects=next(e for e in fbx.elems if e.id==b'Objects')
shapes=[e for e in objects.elems if e.id==b'Geometry' and e.props[2]==b'Shape']
assert len(shapes)==23
models=[e.props[1].split(b'\x00')[0].decode() for e in objects.elems if e.id==b'Model']
assert all(f'Braid.{s}.{i+1:02}' in models for s in ['L','R'] for i in range(3))
morph=json.loads((ASSETS/'naia.morphs.json').read_text())
connections=next(e for e in fbx.elems if e.id==b'Connections')
parents={e.props[1]:e.props[2] for e in connections.elems if e.props[0]==b'OO'}
by_id={e.props[0]:e for e in objects.elems}
# The shared exporter also preserves separate refined hands/sleeves. Resolve
# the body from the blend-shape connection rather than assuming mesh order.
geometry=by_id[parents[parents[parents[shapes[0].props[0]]]]]
vertices=next(e.props[0] for e in geometry.elems if e.id==b'Vertices')
assert len(vertices)==len(morph['vertices'])
assert np.max(np.abs(np.array(vertices)-morph['vertices']))<1e-6
for path in [ASSETS/'naia-rig.json',ASSETS/'manifest.json']:
    d=json.loads(path.read_text(encoding='utf8'));item=next(x for x in d if x['slug']=='naia') if isinstance(d,list) else d
    assert item['bones']==58
for name in ['wink','smile','squeeze','recover']:
    data=(ASSETS/f'naia-attack-{name}.png').read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n'
    offset=8;idat=b'';ended=False
    while offset<len(data):
        length=struct.unpack('>I',data[offset:offset+4])[0];tag=data[offset+4:offset+8]
        payload=data[offset+8:offset+8+length];crc=data[offset+8+length:offset+12+length]
        assert len(crc)==4 and struct.unpack('>I',crc)[0]==zlib.crc32(tag+payload)&0xffffffff,(name,'PNG CRC')
        if tag==b'IDAT':idat+=payload
        if tag==b'IEND':ended=True
        offset+=length+12
    assert ended and len(zlib.decompress(idat))==2048*(2048*4+1),(name,'PNG payload')
report=dict(passed=True,bones=58,morphs=23,face_textures_verified=4,independent_braid_displacement_m=independent,max_braid_edge_stretch=max_edge,attack_frames=101,recovery_seam_m=float(np.max(np.abs(start-end))))
(OUT/'verification.json').write_text(json.dumps(report,indent=2));print('NAIA_VERIFY_OK',json.dumps(report),flush=True)
