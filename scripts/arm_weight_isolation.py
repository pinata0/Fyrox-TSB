"""Keep generated body/hair weights out of the separately authored arm rig.

The refined pipeline replaces both arms with explicit hand/sleeve meshes. The
remaining fused source surface is the head, hair, torso and hanging costume;
its old automatic arm weights are never a valid arm selection mask.
"""
import heapq
import numpy as np
from mathutils.kdtree import KDTree

REVISION = 'separate-arm-domains-v1'


def arm_bones(rig):
    names = set()
    for bone in rig.data.bones:
        parent = bone
        while parent:
            if parent.name in {'UpperArm.L', 'UpperArm.R'}:
                names.add(bone.name)
                break
            parent = parent.parent
    return names


def has_refined_arms(body, rig):
    objects = {obj.name for obj in rig.children}
    slug = body.name.removesuffix('_SkinnedMesh')
    return all(f'{slug}_Refined{part}.{side}' in objects
               for part in ('Hand', 'Sleeve') for side in ('L', 'R'))


def normalize(row):
    row = sorted(((n, w) for n, w in row.items() if w > 1e-7),
                 key=lambda pair: (-pair[1], pair[0]))[:4]
    total = sum(w for _, w in row)
    if not total:
        raise ValueError('Cannot normalize an empty skin assignment')
    return {n: float(w / total) for n, w in row}


def assign(obj, index, row):
    for group in list(obj.data.vertices[index].groups):
        obj.vertex_groups[group.group].remove([index])
    for name, weight in normalize(row).items():
        group = obj.vertex_groups.get(name)
        if group is None:
            group = obj.vertex_groups.new(name=name)
        group.add([index], weight, 'REPLACE')


def isolate_body(body, rig):
    """Extend reliable non-arm weights along the surface into polluted regions.

    Existing non-arm blends remain intact where their mass is >= 25%. Below
    that threshold, use surface distance to reliable anchors, then relax only
    the uncertain region. Spatial proximity is used only for disconnected
    fragments/unused vertices, never to shortcut across a nearby hair strand.
    """
    excluded = arm_bones(rig)
    rows = [{body.vertex_groups[g.group].name: g.weight for g in v.groups}
            for v in body.data.vertices]
    affected = np.array([any(n in excluded and w > 0 for n, w in r.items()) for r in rows])
    if not affected.any():
        return {'changed_vertices': 0, 'reconstructed_vertices': 0}
    points = np.array([v.co[:] for v in body.data.vertices])
    names = sorted({n for r in rows for n in r if n not in excluded})
    columns = {n: i for i, n in enumerate(names)}
    values = np.zeros((len(rows), len(names)))
    for i, row in enumerate(rows):
        for n, w in row.items():
            if n in columns:
                values[i, columns[n]] = w
    confidence = values.sum(axis=1)
    values /= np.maximum(confidence[:, None], 1e-12)
    anchors = confidence >= .25
    if not anchors.any():
        raise ValueError(f'{body.name}: no reliable non-arm anchors')
    neighbors = [[] for _ in rows]
    for edge in body.data.edges:
        a, b = edge.vertices
        length = float(np.linalg.norm(points[a] - points[b]))
        neighbors[a].append((b, length))
        neighbors[b].append((a, length))
    distance = np.full(len(rows), np.inf)
    owner = np.full(len(rows), -1, dtype=int)
    queue = []
    for i in np.flatnonzero(anchors):
        i = int(i)
        distance[i] = 0
        owner[i] = i
        queue.append((0., i))
    heapq.heapify(queue)
    while queue:
        dist, i = heapq.heappop(queue)
        if dist > distance[i]:
            continue
        for j, length in neighbors[i]:
            candidate = dist + length
            if candidate < distance[j]:
                distance[j] = candidate
                owner[j] = owner[i]
                heapq.heappush(queue, (candidate, j))
    tree = KDTree(int(anchors.sum()))
    for i in np.flatnonzero(anchors):
        tree.insert(points[i], int(i))
    tree.balance()
    for i in np.flatnonzero(owner < 0):
        owner[i] = tree.find(points[i])[1]
    uncertain = np.flatnonzero(~anchors)
    values[uncertain] = values[owner[uncertain]]
    # Jacobi smoothing avoids hard ownership seams in fully arm-weighted hair.
    dst, src, conductance = [], [], []
    for i in uncertain:
        for j, length in neighbors[i]:
            dst.append(i); src.append(j); conductance.append(1 / max(length, 1e-5))
    dst, src = np.array(dst, dtype=int), np.array(src, dtype=int)
    conductance = np.array(conductance)
    totals = np.bincount(dst, weights=conductance, minlength=len(rows))
    active = np.flatnonzero(totals > 0)
    for _ in range(64):
        average = np.zeros_like(values)
        for column in range(len(names)):
            average[:, column] = np.bincount(
                dst, weights=values[src, column] * conductance, minlength=len(rows))
        values[active] = .5 * values[active] + .5 * average[active] / totals[active, None]
    for i in np.flatnonzero(affected):
        assign(body, int(i), dict(zip(names, values[i])))
    return {'changed_vertices': int(affected.sum()),
            'reconstructed_vertices': int((affected & ~anchors).sum())}


def smoothstep(value):
    value = np.clip(value, 0, 1)
    return value * value * (3 - 2 * value)


def isolate_restored_coat(coat, rig):
    """Retain the real coat sleeves; fade the hanging panels to torso/pelvis."""
    if coat.get('arm_weight_isolation') == REVISION:
        return {'vertices': len(coat.data.vertices), 'sleeve_core_radius_m': .085,
                'sleeve_outer_radius_m': .19}
    excluded = arm_bones(rig)
    points = np.array([v.co[:] for v in coat.data.vertices])
    influence = np.zeros((len(points), 2))
    for side_index, side in enumerate(('L', 'R')):
        joints = [np.array(rig.data.bones[f'{n}.{side}'].head_local)
                  for n in ('UpperArm', 'Forearm', 'Hand')]
        distance = np.full(len(points), np.inf)
        for start, end in zip(joints, joints[1:]):
            axis = end - start
            t = np.clip((points - start) @ axis / np.dot(axis, axis), 0, 1)
            distance = np.minimum(distance, np.linalg.norm(points - start - t[:, None] * axis, axis=1))
        influence[:, side_index] = 1 - smoothstep((distance - .085) / .105)
    torso_z = rig.data.bones['Torso'].head_local.z
    hips_z = rig.data.bones['Hips'].head_local.z
    pelvis = smoothstep((torso_z - points[:, 2]) / max(.15, torso_z - hips_z + .15))
    for vertex in coat.data.vertices:
        row = {}
        for g in vertex.groups:
            name = coat.vertex_groups[g.group].name
            if name not in excluded:
                continue
            side = name.rsplit('.', 1)[-1]
            if side not in ('L', 'R'):
                continue
            # Cuffs stay on the forearm when fingers/wrist rotate around a grip.
            target = name if name.startswith(('UpperArm.', 'Forearm.')) else f'Forearm.{side}'
            w = g.weight * influence[vertex.index, ('L', 'R').index(side)]
            row[target] = row.get(target, 0) + w
        remainder = max(0., 1 - sum(row.values()))
        row['Torso'] = remainder * (1 - pelvis[vertex.index])
        row['Hips'] = remainder * pelvis[vertex.index]
        assign(coat, vertex.index, row)
    coat['arm_weight_isolation'] = REVISION
    return {'vertices': len(points), 'sleeve_core_radius_m': .085,
            'sleeve_outer_radius_m': .19}


def isolate_refined_arm_weights(body, rig):
    if not has_refined_arms(body, rig):
        raise ValueError('Arm isolation requires both replacement hands and sleeves')
    report = isolate_body(body, rig)
    for obj in rig.children:
        if obj.type == 'MESH' and obj.name.endswith('_RestoredCoat'):
            report['restored_coat'] = isolate_restored_coat(obj, rig)
    body['arm_weight_isolation'] = REVISION
    return report
