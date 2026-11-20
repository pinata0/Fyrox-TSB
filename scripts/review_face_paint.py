"""Orthographic texture/cheek-weight diagnostics without lighting or animation."""
import bpy
import json
import sys
from pathlib import Path

import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/characters'
OUT = ROOT / 'artifacts/face-feature-fix'
OUT.mkdir(parents=True, exist_ok=True)
sys.dont_write_bytecode = True


def project(body, face, size=384):
    mesh = body.data
    points = np.array([v.co[:] for v in mesh.vertices])
    mesh.calc_loop_triangles()
    tris = list(mesh.loop_triangles)
    uv = np.array([v.uv[:] for v in mesh.uv_layers.active.data])
    tree = BVHTree.FromPolygons(points.tolist(), [t.vertices[:] for t in tris], all_triangles=True)
    xs = np.linspace(face['center_x']-.43, face['center_x']+.43, size)
    zs = np.linspace(face['mouth_y']-.16, face['eye_y']+.20, size)
    hit_tri = np.full((size, size), -1)
    positions = np.zeros((size, size, 3))
    for iz, z in enumerate(zs):
        for ix, x in enumerate(xs):
            hit, _, index, _ = tree.ray_cast(Vector((x, -3, z)), Vector((0, 1, 0)))
            if hit is not None:
                hit_tri[iz, ix] = index
                positions[iz, ix] = hit
    sampled_uv = np.zeros((size, size, 2))
    for index in np.unique(hit_tri[hit_tri >= 0]):
        mask = hit_tri == index
        tri = tris[index]
        p = points[list(tri.vertices)]
        bary = np.linalg.lstsq(np.vstack([p.T, np.ones(3)]), np.column_stack((positions[mask], np.ones(mask.sum()))).T, rcond=None)[0].T
        sampled_uv[mask] = bary @ uv[list(tri.loops)]
    return positions, sampled_uv, hit_tri


def sample(path, uv):
    image = bpy.data.images.load(str(path), check_existing=False)
    w, h = image.size
    pixels = np.array(image.pixels[:], dtype=np.float32).reshape(h, w, 4)
    result = pixels[np.clip((uv[..., 1]*h).astype(int), 0, h-1), np.clip((uv[..., 0]*w).astype(int), 0, w-1)]
    bpy.data.images.remove(image)
    return result


if __name__ == '__main__':
    args = sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else ['komi', 'vivi', 'chloe']
    bake_regions = '--bake-regions' in args
    region_path = ASSETS / 'blush-regions.json'
    regions = json.loads(region_path.read_text()) if region_path.exists() else {}
    for slug in [arg for arg in args if not arg.startswith('--')]:
        bpy.ops.wm.open_mainfile(filepath=str(ASSETS / f'{slug}.blend'))
        body = bpy.data.objects[f'{slug}_SkinnedMesh']
        face = json.loads((ASSETS / f'{slug}-rig.json').read_text())['face']
        pos, uv, hit_tri = project(body, face)
        valid = hit_tri >= 0
        colors = [sample(ASSETS / f'{slug}{suffix}.png', uv) for suffix in ['-meshy-basecolor', '-blink-half', '-blink-closed']]
        for color in colors:
            color[~valid] = [.2, .2, .2, 1]
        pixels = np.concatenate(colors, axis=1)
        image = bpy.data.images.new('review', width=pixels.shape[1], height=pixels.shape[0], alpha=True)
        image.pixels.foreach_set(pixels.ravel())
        image.filepath_raw = str(OUT / f'{slug}-paint.png')
        image.file_format = 'PNG'
        image.save()
        np.savez_compressed(OUT / f'{slug}-projection.npz', pos=pos, uv=uv, valid=valid, hit_tri=hit_tri, base=colors[0])
        if bake_regions:
            from face_regions import blush_profiles
            regions[slug] = blush_profiles(pos, colors[0], valid, face, slug)
        print('FACE_PAINT_REVIEW_OK', slug, flush=True)
    if bake_regions:
        region_path.write_text(json.dumps(regions, indent=2))
