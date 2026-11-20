"""Bake eye-safe cheek controls and restore round blush under closed eyes.

Only runtime cheek controls and blink atlases are written. Meshes, rigs, neutral
textures and animation data are read-only, including concurrent character edits.
"""
import bpy
import json
import shutil
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.dont_write_bytecode = True
from face_regions import eye_triangles
from build_cheek_masks import distances, smooth

ASSETS = ROOT / 'assets/characters'
OUT = ROOT / 'artifacts/face-feature-fix'
DEST = OUT / 'candidate'
DEST.mkdir(parents=True, exist_ok=True)
PROFILES = json.loads((ASSETS / 'blush-regions.json').read_text())


def repair_controls(mesh, points, face, slug):
    record = json.loads((ASSETS / f'{slug}-cheeks.json').read_text())
    rows = np.array(record['vertices'])
    assert np.allclose(points, rows[:, :3], atol=1e-6)
    if record.get('eye_triangle_guard', {}).get('revision') == 1:
        (DEST / f'{slug}-cheeks.json').write_text(json.dumps(record, separators=(',', ':')))
        return record['eye_triangle_guard']
    mesh.calc_loop_triangles()
    triangles = np.array([t.vertices[:] for t in mesh.loop_triangles])
    eyes = eye_triangles(points, triangles, face)
    adjacency = [[] for _ in points]
    for edge in mesh.edges:
        a, b = edge.vertices
        length = float(np.linalg.norm(points[a]-points[b]))
        adjacency[a].append((int(b), length))
        adjacency[b].append((int(a), length))
    clearance = distances(points, adjacency, np.flatnonzero(eyes))
    guard = smooth(clearance/.027)
    old = rows[:, 3:5].copy()
    rows[:, 3:5] *= guard[:, None]
    rows[eyes, 3:11] = 0
    for side in range(2):
        weight = rows[:, 3+side]
        weight[weight < 1e-6] = 0
        gradient = np.zeros_like(points)
        for i in np.flatnonzero(weight > 0):
            neighbors = [j for j, _ in adjacency[i]]
            delta = points[neighbors]-points[i]
            gradient[i] = np.linalg.lstsq(np.vstack([delta, np.eye(3)*.005]), np.r_[weight[neighbors]-weight[i], np.zeros(3)], rcond=None)[0]
        rows[:, 5+side*3:8+side*3] = np.round(gradient, 5)
        assert (weight > .2).sum() > 25, (slug, side, 'cheek must still move')
    rows[:, 3:5] = np.round(rows[:, 3:5], 6)
    rows[eyes, 11] = rows[eyes, 11].astype(int) | 1 | 8
    assert np.all(rows[eyes, 3:11] == 0)
    record['vertices'] = rows.tolist()
    record['eye_triangle_guard'] = dict(revision=1, pinned_vertices=int(eyes.sum()), newly_pinned_moving_vertices=int((eyes & (old.max(axis=1) > 0)).sum()))
    (DEST / f'{slug}-cheeks.json').write_text(json.dumps(record, separators=(',', ':')))
    return record['eye_triangle_guard']


if __name__ == '__main__':
    selected = sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else list(PROFILES)
    report = {}
    for slug in selected:
        before = OUT / 'before'
        before.mkdir(exist_ok=True)
        for suffix in ['-cheeks.json', '-blink-L.png', '-blink-R.png', '-blink-half.png', '-blink-closed.png']:
            source = ASSETS / f'{slug}{suffix}'
            if not (before / source.name).exists():
                shutil.copy2(source, before / source.name)
        bpy.ops.wm.open_mainfile(filepath=str(ASSETS / f'{slug}.blend'))
        mesh = bpy.data.objects[f'{slug}_SkinnedMesh'].data
        points = np.array([v.co[:] for v in mesh.vertices])
        face = json.loads((ASSETS / f'{slug}-rig.json').read_text())['face']
        report[slug] = repair_controls(mesh, points, face, slug)
        import import_meshy_characters as painter
        painter.STAGE = DEST
        painter.paint_blink_textures(bpy.data.objects[f'{slug}_SkinnedMesh'], face, slug, build_nodes=False, natural=True)
        report[slug]['regenerated_blink_textures'] = 4
        print('FACE_FEATURE_FIX_OK', slug, report[slug], flush=True)
    report_name = 'bake-report.json' if len(selected) == len(PROFILES) else f'bake-report-{selected[0]}.json'
    (OUT / report_name).write_text(json.dumps(report, indent=2))
