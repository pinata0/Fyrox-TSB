"""Build and verify blink fixes in a candidate directory before publication."""
import bpy,json,sys,shutil,os
import numpy as np
from pathlib import Path
from mathutils.bvhtree import BVHTree

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from natural_eyelids import sample_lids
from blink_surfaces import fit_surfaces,rebuild_blink_shapes,protected_vertices
from export_fyrox_fbx import export_fbx,resolve_character_textures
import import_meshy_characters as painter
ASSETS=ROOT/'assets/characters'
OUT=ROOT/'artifacts/blink-surface-fix'
DEST=OUT/'candidate'
painter.STAGE=DEST
SLUGS='vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()

for slug in (sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS):
    for suffix in ['.blend','.fbx','.morphs.json','-rig.json','-blink-L.png','-blink-R.png','-blink-half.png','-blink-closed.png']:
        source=ASSETS/f'{slug}{suffix}';backup=OUT/'before'/source.name
        if not backup.exists():shutil.copy2(source,backup)
    geometry_only=os.environ.get('TSB_BLINK_GEOMETRY_ONLY')=='1'
    source=(DEST if geometry_only else ASSETS)/f'{slug}.blend'
    bpy.ops.wm.open_mainfile(filepath=str(source))
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];mesh=body.data;rig=bpy.data.objects['TSB_Rig']
    for shape in mesh.shape_keys.key_blocks:shape.value=0
    face=json.loads((ASSETS/f'{slug}-rig.json').read_text())['face']
    resolve_character_textures([o for o in bpy.context.scene.objects if o.type=='MESH'])
    for image in bpy.data.images:
        if image.source=='FILE' and image.filepath:image.filepath=bpy.path.abspath(image.filepath)
    points=np.array([v.co[:] for v in mesh.vertices]);mesh.calc_loop_triangles();tris=list(mesh.loop_triangles)
    uvs=np.array([u.uv[:] for u in mesh.uv_layers.active.data])
    image=bpy.data.images.load(str(ASSETS/f'{slug}-meshy-basecolor.png'),check_existing=False)
    w,h=image.size;pixels=np.array(image.pixels[:],dtype=np.float32).reshape(h,w,4)
    tree=BVHTree.FromPolygons(points.tolist(),[t.vertices[:] for t in tris],all_triangles=True)
    profiles=sample_lids(tree,tris,points,uvs,pixels,face,skip=('R',) if slug=='aya' else ())
    fit_surfaces(points,profiles,face,protected_vertices(body))
    stats=rebuild_blink_shapes(body,face,profiles)
    if not geometry_only:painter.paint_blink_textures(body,face,slug,build_nodes=False,natural=True)
    bpy.context.scene['blink_surface_revision']='smooth-depth-and-normal-v1'
    bpy.context.scene.frame_set(0)
    for shape in mesh.shape_keys.key_blocks:shape.value=0
    assert np.array_equal(points,np.array([v.co[:] for v in mesh.vertices]))
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(DEST/f'{slug}.blend'),relative_remap=False)
    export_fbx(body,rig,DEST/f'{slug}.fbx')
    (OUT/f'{slug}-surface.json').write_text(json.dumps(stats,indent=2))
    print('BLINK_SURFACE_OK',slug,stats,flush=True)
