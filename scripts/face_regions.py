"""Recover complete eye outlines and round cheek paint from the neutral atlas."""
from collections import deque
import numpy as np

# Measured against the 384px front projection used by review_face_paint.py.
# These complete the hidden arc rather than fitting hair/ear/glasses occlusion.
# (center x, center y, horizontal radius, vertical radius), L then R.
BLUSH_OUTLINES = {
    'aya': [(129, 236, 25, 50)],
    'bella': [(114, 244, 33, 58), (249, 243, 30, 59)],
    'butter': [(104, 254, 38, 54), (278, 254, 38, 54)],
    'chloe': [(113, 258, 32, 56), (282, 255, 33, 59)],
    'erpin': [(99, 232, 45, 60), (286, 232, 45, 60)],
    'hilde': [(110, 242, 36, 56), (274, 241, 35, 56)],
    'id': [],
    'komi': [(86, 249, 47, 65), (286, 250, 43, 64)],
    'mayo': [(114, 226, 32, 48), (274, 226, 32, 48)],
    'naia': [(113, 235, 40, 50), (269, 235, 40, 50)],
    'picora': [(86, 243, 52, 62), (299, 244, 53, 61)],
    'renua': [(112, 245, 28, 46), (266, 245, 28, 46)],
    'shion': [(91, 236, 44, 67), (292, 236, 44, 67)],
    'ui': [(110, 272, 37, 59), (282, 272, 38, 59)],
    'vivi': [(112, 229, 28, 50), (273, 229, 29, 50)],
}


def components(mask):
    remaining = set(map(tuple, np.argwhere(mask)))
    result = []
    while remaining:
        first = remaining.pop()
        part = [first]
        queue = deque([first])
        while queue:
            y, x = queue.popleft()
            for neighbor in [(y-1, x), (y+1, x), (y, x-1), (y, x+1)]:
                if neighbor in remaining:
                    remaining.remove(neighbor)
                    queue.append(neighbor)
                    part.append(neighbor)
        result.append(np.array(part))
    return result


def ellipse(points):
    # The exposed outer extremes survive the eye overlap. Keep an upright
    # round contour instead of fitting an unstable conic to the missing arc.
    center = (points.min(0)+points.max(0))/2
    radii = np.maximum(np.ptp(points, axis=0)/2, .01)
    return dict(center=center.tolist(), matrix=np.diag(1/radii**2).tolist())


def blush_profiles(pos, base, valid, face, slug=None):
    x, z = pos[..., 0], pos[..., 2]
    r, g, b = [base[..., i] for i in range(3)]
    skin_sample = valid & (np.abs(x-face['center_x']) < face['eye_x']*.35)
    skin_sample &= (z > face['mouth_y']+.025) & (z < face['eye_y']-face['eye_half_height']*.35)
    skin_ratio = np.median((g/np.maximum(r, .001))[skin_sample]) if skin_sample.any() else .94
    pink = (r > .9) & (g < r*min(.86, skin_ratio-.06)) & (b > g*.90)
    profiles = {}
    for side, sign in [('L', -1), ('R', 1)]:
        roi = valid & (sign*(x-face['center_x']) > face['eye_x']*.65)
        roi &= (z < face['eye_y']-face['eye_half_height']*.35) & (z > face['mouth_y']-.12)
        parts = components(pink & roi)
        if not parts:
            continue
        part = max(parts, key=len)
        if len(part) < 80:
            continue
        mask = np.zeros_like(valid)
        mask[tuple(part.T)] = True
        points = pos[mask][:, [0, 2]]
        profile = ellipse(points)
        # Hair and ears often hide or extend the outer edge. Recover the
        # center from the exposed bottom arc and the radius from the inner
        # edge of the lower half, where the open eye cannot occlude the paint.
        low, high = points[:, 1].min(), points[:, 1].max()
        bottom = points[points[:, 1] < low+(high-low)*.18]
        center_x = float(np.mean(bottom[:, 0]))
        middle = points[(points[:, 1] > low+(high-low)*.35) & (points[:, 1] < low+(high-low)*.55)]
        if len(middle):
            inner = middle[:, 0].max() if sign < 0 else middle[:, 0].min()
            radius_x = abs(inner-center_x)
            radius_z = max((high-low)/2, radius_x*1.08)
            if .025 < radius_x < .14:
                profile = dict(center=[center_x, low+radius_z], matrix=np.diag(1/np.array([radius_x, radius_z])**2).tolist())
        profile['color'] = np.median(base[mask, :3], axis=0).tolist()
        profile['depth'] = float(np.median(pos[mask, 1]))
        profile['pixels'] = int(mask.sum())
        profiles[side] = profile
    if slug in BLUSH_OUTLINES:
        calibrated = {}
        for side, (px, py, rx, ry) in zip(['L', 'R'], BLUSH_OUTLINES[slug]):
            scale = np.array([.86, face['eye_y']+.20-(face['mouth_y']-.16)])/383
            center = [face['center_x']-.43+px*scale[0], face['eye_y']+.20-py*scale[1]]
            matrix = np.diag(1/(np.array([rx, ry])*scale)**2)
            delta = pos[:, :, [0, 2]]-center
            interior = np.einsum('...i,ij,...j->...', delta, matrix, delta) < .9
            painted = valid & pink & interior
            if painted.sum() < 20:
                raise ValueError(f'{slug}/{side}: missing cheek paint inside authored outline')
            calibrated[side] = dict(center=center, matrix=matrix.tolist(), color=np.median(base[painted, :3], axis=0).tolist(), outline_pixels=[px, py, rx, ry])
        return calibrated
    return profiles


def blush_layer(pos, profiles, skin):
    color = np.tile(skin, (len(pos), 1))
    coverage = np.zeros(len(pos))
    for profile in profiles.values():
        d = pos[:, [0, 2]]-profile['center']
        q = np.einsum('ni,ij,nj->n', d, np.array(profile['matrix']), d)
        alpha = np.clip((1.035-q)/.07, 0, 1)
        # The original eye is often a raised sheet in front of the cheek.
        # Blink's own surface/accessory mask chooses the visible face; clipping
        # this underpaint to cheek depth would leave a crescent at the eye rim.
        color = color*(1-alpha[:, None])+np.array(profile['color'])*alpha[:, None]
        coverage = np.maximum(coverage, alpha)
    return color, coverage


def eye_triangles(points, triangles, face):
    """Pin entire triangles crossing the eye, including their lower/outer rim.

    A vertex-only ellipse lets a triangle stretch painted eye texels between
    protected vertices. The expanded bounds also cover the full illustrated
    lower lid, which extends below the old eye-height landmark.
    """
    p = points[triangles]
    low, high = p.min(axis=1), p.max(axis=1)
    result = np.zeros(len(points), dtype=bool)
    ex, ez, eh, cx = [face[k] for k in ['eye_x', 'eye_y', 'eye_half_height', 'center_x']]
    # The narrower lower half preserves the cheek beside the tapered iris.
    for sign in [-1, 1]:
        center = cx+sign*ex
        for bottom, top, width in [(-1.35, -.35, .57), (-.35, 1.65, 1.10)]:
            touched = (high[:, 0] >= center-ex*width-.006) & (low[:, 0] <= center+ex*width+.006)
            touched &= (high[:, 2] >= ez+eh*bottom-.006) & (low[:, 2] <= ez+eh*top+.006)
            result[triangles[touched].ravel()] = True
    return result
