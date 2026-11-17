"""Blender background QA: the saved editable file must match the shared solids."""
import bpy, hashlib, json
from pathlib import Path

root=Path(__file__).resolve().parents[1]
out=root/'assets/maps/elf-city'
layout=json.loads((out/'layout.json').read_text(encoding='utf-8'))
bpy.ops.wm.open_mainfile(filepath=str(out/'elf-city.blend'))
scene=bpy.context.scene
solids=[o for o in scene.objects if o.get('collision')]
expected=[b for b in layout['blocks'] if b.get('render',True)]
assert len(solids)==len(expected),(len(solids),len(expected))
def close(a,b):return all(abs(x-y)<.001 for x,y in zip(a,b))
for b in expected:
    p=b['center'];s=b['size']
    matches=[o for o in solids if close(o.location,(p[0],-p[2],p[1])) and close(o.dimensions,(s[0],s[2],s[1]))]
    assert matches,b['name']
meshes=[o for o in scene.objects if o.type=='MESH']
assert all(o.data.uv_layers for o in meshes)
assert all(p.area>1e-10 for o in meshes for p in o.data.polygons)
assert scene['map_slug']==layout['slug']=='elf-city'
assert (out/'elf-city.fbx').stat().st_size>100000
report=json.loads((out/'build-report.json').read_text(encoding='utf-8'))
assert report['objects']==len(meshes)
assert report['collision_blocks']==len(layout['blocks'])
report['materials']=sum('srgb' in m for m in bpy.data.materials)
report['verified_visible_collision_blocks']=len(expected)
report['layout_sha256']=hashlib.sha256((out/'layout.json').read_bytes()).hexdigest()
report['mesh_qa']='Nonzero faces, UVs, Blender solid positions/dimensions match layout.'
(out/'build-report.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print('ELF_ASSET_QA_OK',json.dumps(report),flush=True)
