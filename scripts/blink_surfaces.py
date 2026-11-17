"""Smooth closed-lid surfaces and a shared eye/outer-corner footprint.

The neutral mesh and UVs stay intact. Only eyeBlink's depth and normals change;
texture erasure uses the same front-surface bounds, including the lash wing.
"""
import numpy as np
import json
from pathlib import Path


def protected_vertices(body):
    """Reuse the authored glasses/hair-clip guards, keyed across UV duplicates."""
    slug=body.name.removesuffix('_SkinnedMesh')
    result=np.zeros(len(body.data.vertices),dtype=bool)
    if slug not in ['hilde','ui']:return result
    path=Path(__file__).resolve().parents[1]/'assets/characters'/f'{slug}-cheeks.json'
    records=json.loads(path.read_text())['vertices']
    guarded={tuple(round(v,5) for v in row[:3]) for row in records if int(row[-1])&4}
    result=np.array([tuple(round(v,5) for v in vertex.co) in guarded for vertex in body.data.vertices])
    if slug=='hilde':
        # The cheek guard deliberately includes all foreground features. For
        # blinking, isolate the pale green/grey rim from dark lashes and pink
        # skin in that guard; keeping the lashes would leave old corner ink.
        import bpy
        image=bpy.data.images.load(str(path.parent/f'{slug}-meshy-basecolor.png'),check_existing=False)
        w,h=image.size;pixels=np.array(image.pixels[:],dtype=np.float32).reshape(h,w,4)
        uv=np.array([v.uv[:] for v in body.data.uv_layers.active.data])
        indices=np.array([v.vertex_index for v in body.data.loops])
        sampled=pixels[np.clip((uv[:,1]*h).astype(int),0,h-1),np.clip((uv[:,0]*w).astype(int),0,w-1),:3]
        colors=np.zeros((len(result),3));count=np.zeros(len(result))
        np.add.at(colors,indices,sampled);np.add.at(count,indices,1)
        colors/=np.maximum(count[:,None],1)
        r,g,b=colors.T
        pale=(colors.min(axis=1)>.48)&(g>=r*.98)&(g>=b*.98)
        # Flood the neutral-coloured foreground hair from unambiguous fringe
        # and side-hair seeds. Colour alone would also protect grey antialiasing
        # on the old eyelashes and leave a dotted open-eye outline.
        from collections import deque
        face=json.loads((path.parent/f'{slug}-rig.json').read_text())['face']
        points=np.array([v.co[:] for v in body.data.vertices])
        x,y,z=points.T
        neutral=(colors.min(axis=1)>.18)&(colors.max(axis=1)<.65)&(np.ptp(colors,axis=1)<.13)
        seeds=neutral&((np.abs(x-face['center_x'])<face['eye_x']*.30)
                       |(np.abs(x-face['center_x'])>face['eye_x']*2.40)
                       |(z>face['eye_y']+face['eye_half_height']*1.65))
        adjacency=[[] for _ in points]
        for edge in body.data.edges:
            a,b=edge.vertices;adjacency[a].append(b);adjacency[b].append(a)
        hair=seeds.copy();queue=deque(np.flatnonzero(hair))
        while queue:
            for j in adjacency[queue.popleft()]:
                if neutral[j] and not hair[j]:hair[j]=True;queue.append(j)
        result=(result&pale)|hair
        bpy.data.images.remove(image)
    return result


def smooth(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)


def terms(x, z):
    return np.column_stack((np.ones_like(x), x, z, x*x, x*z, z*z))


def fit_surfaces(points, profiles, face, protected=None):
    x, y, z = points.T
    eh = face['eye_half_height']
    for profile in profiles.values():
        lo, hi = profile['span']
        center = (lo + hi) / 2
        scale = (hi - lo) / 2
        upper = np.interp(x, profile['x'], profile['open'])
        depth = np.interp(x, profile['x'], profile['depth'])
        # Include the surrounding skin, but not hair sheets in front of the face.
        candidates = ((x > lo-.025) & (x < hi+.025)
                      & (z > face['eye_y']-eh*1.35) & (z < upper+.018)
                      & (np.abs(y-depth) < .045))
        if protected is not None:candidates &= ~protected
        u = (x-center)/scale
        v = (z-face['eye_y'])/eh
        matrix = terms(u, v)
        selected = np.flatnonzero(candidates)
        if len(selected) < 30:
            raise ValueError('Insufficient visible face surface for closed lid')
        a, b = matrix[selected], y[selected]
        # Robust quadratic regression removes the narrow raised eye rim while
        # retaining the broad curvature of each side of the face.
        weights = np.ones(len(selected))
        for _ in range(6):
            coeff = np.linalg.lstsq(a*weights[:,None], b*weights, rcond=None)[0]
            residual = np.abs(a@coeff-b)
            weights = np.minimum(1, .008/np.maximum(residual, 1e-8))
        profile['surface'] = dict(coeff=coeff, center=center, scale=scale,
                                  eye_y=face['eye_y'], eh=eh)
    return profiles


def surface_at(profile, points):
    f = profile['surface']
    u = (points[:,0]-f['center'])/f['scale']
    v = (points[:,2]-f['eye_y'])/f['eh']
    c = f['coeff']
    depth = terms(u,v)@c
    dx = (c[1]+2*c[3]*u+c[4]*v)/f['scale']
    dz = (c[2]+c[4]*u+2*c[5]*v)/f['eh']
    normals = np.column_stack((dx, -np.ones(len(u)), dz))
    normals /= np.linalg.norm(normals, axis=1)[:,None]
    return depth, normals


def footprint(profile, points, geometry=False):
    x,y,z = points.T
    lo,hi = profile['span']
    f = profile['surface']
    upper = np.interp(x,profile['x'],profile['open'])
    thickness = np.interp(x,profile['x'],profile['source_thickness'])
    bottom = f['eye_y']-f['eh']*1.28
    # A rectangular wing extension includes the triangular outer lash below
    # the sampled band, which an ellipse or a thin band cannot cover.
    pad,fade = (.030,.026) if geometry else (.010,.008)
    side = smooth((x-lo+pad)/fade)*smooth((hi+pad-x)/fade)
    vertical = smooth((z-bottom+pad)/fade)
    vertical *= smooth((upper+thickness*.60+pad-z)/fade)
    depth,_ = surface_at(profile,points)
    front = 1-smooth((np.abs(y-depth)-.022)/.025)
    return side*vertical*front


def rebuild_blink_shapes(body, face, profiles):
    mesh = body.data
    basis = np.array([v.co[:] for v in mesh.shape_keys.key_blocks[0].data])
    stats = {}
    for side,profile in profiles.items():
        depth,normals = surface_at(profile,basis)
        weight = footprint(profile,basis,geometry=True)
        weight[protected_vertices(body)]=0
        target = basis.copy()
        # Meshy can contain front/back eye sheets at the same projected point.
        # Retain a small fraction of their depth separation to prevent z-fighting
        # and zero-thickness side walls when both sheets approach the lid.
        target[:,1] += (depth-basis[:,1])*weight*.995
        mesh.shape_keys.key_blocks[f'eyeBlink_{side}'].data.foreach_set('co',target.astype(np.float32).ravel())
        for name,kind,values,prop in [
            (f'blink_normal_{side}','FLOAT_VECTOR',normals,'vector'),
            (f'blink_weight_{side}','FLOAT',weight,'value'),
        ]:
            attr = mesh.attributes.get(name) or mesh.attributes.new(name,kind,'POINT')
            attr.data.foreach_set(prop,values.astype(np.float32).ravel())
        moved = np.abs(target[:,1]-basis[:,1])
        stats[side] = dict(vertices=int(np.count_nonzero(weight>.001)),
                           max_depth_change=float(moved.max()),
                           rim_residual_before=float(np.sqrt(np.mean((basis[weight>.99,1]-depth[weight>.99])**2))))
    mesh.update()
    return stats
