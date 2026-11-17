"""Capture front-facing eye geometry and texture for blink repair diagnostics."""
import bpy, json, sys
import numpy as np
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/blink-surface-fix'
sys.path.insert(0, str(ROOT / 'scripts'))
from natural_eyelids import sample_lids

for slug in sys.argv[sys.argv.index('--') + 1:]:
    bpy.ops.wm.open_mainfile(filepath=str(ROOT / f'assets/characters/{slug}.blend'))
    mesh = bpy.data.objects[f'{slug}_SkinnedMesh'].data
    face = json.loads((ROOT / f'assets/characters/{slug}-rig.json').read_text())['face']
    mesh.calc_loop_triangles()
    points = np.array([v.co[:] for v in mesh.vertices])
    tris = list(mesh.loop_triangles)
    uv = np.array([u.uv[:] for u in mesh.uv_layers.active.data])
    tex = bpy.data.images.load(str(ROOT / f'assets/characters/{slug}-meshy-basecolor.png'))
    w,h = tex.size
    pixels = np.array(tex.pixels[:],dtype=np.float32).reshape(h,w,4)
    tree = BVHTree.FromPolygons(points.tolist(), [t.vertices[:] for t in tris], all_triangles=True)
    profiles = sample_lids(tree,tris,points,uv,pixels,face,skip=('R',) if slug=='aya' else ())
    xs=np.linspace(face['center_x']-.38,face['center_x']+.38,480)
    zs=np.linspace(face['mouth_y']-.04,face['eye_y']+face['eye_half_height']*2,300)
    pos=np.full((len(zs),len(xs),3),np.nan);rgb=np.ones_like(pos);normals=np.zeros_like(pos)
    for iz,z in enumerate(zs):
        for ix,x in enumerate(xs):
            hit,normal,index,_=tree.ray_cast(Vector((x,-3,z)),Vector((0,1,0)))
            if hit is None:continue
            p=points[list(tris[index].vertices)];u=uv[list(tris[index].loops)]
            bary=np.linalg.lstsq(np.vstack([p.T,np.ones(3)]),np.r_[hit,1],rcond=None)[0]
            at=bary@u
            rgb[iz,ix]=pixels[min(h-1,max(0,int(at[1]*h))),min(w-1,max(0,int(at[0]*w))),:3]
            pos[iz,ix]=hit;normals[iz,ix]=normal
    np.savez_compressed(OUT/f'{slug}-projection.npz',points=pos,color=rgb,normals=normals)
    img=bpy.data.images.new(slug+' projection',width=len(xs),height=len(zs),alpha=True)
    img.pixels.foreach_set(np.concatenate([rgb,np.ones((*rgb.shape[:2],1))],axis=2).astype(np.float32).ravel())
    img.filepath_raw=str(OUT/f'{slug}-projection.png');img.file_format='PNG';img.save()
    print('EYE_PROFILES',slug,[(s,dict(span=list(p['span']),open=float(np.median(p['open'])),depth=np.quantile(p['depth'],[0,.5,1]).tolist())) for s,p in profiles.items()],flush=True)
