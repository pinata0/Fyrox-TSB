"""Shared outer-gate opening profile for Blender meshes and collision proxies."""
import math

ARCH = dict(spring=3.0, rise=2.0, segments=64)
PROXY_NAME = 'Outer gate arch collision'


def arch_profile(width, arch):
    """Left-to-right elliptical soffit, in horizontal/vertical metres."""
    return [(-width / 2 * math.cos(i * math.pi / arch['segments']),
             arch['spring'] + arch['rise'] * math.sin(i * math.pi / arch['segments']))
            for i in range(arch['segments'] + 1)]


def configure_outer_gate(layout):
    # Append proxies so all existing collision indices (including 052) stay stable.
    blocks = layout['blocks']
    blocks[:] = [b for b in blocks if b['name'] != PROXY_NAME]
    gate = next(g for g in layout['gates'] if g['name'] == 'Outer crown gate')
    assert gate['axis'] == 'z'
    index, lintel = next((i, b) for i, b in enumerate(blocks)
                         if b['name'] == 'Outer gate wall lintel'
                         and b['center'][0] == gate['x'] and b['center'][2] == gate['z'])
    gate['arch'] = dict(ARCH)
    lintel['arch'] = dict(ARCH)
    top = gate['height']
    apex = ARCH['spring'] + ARCH['rise']
    lintel['center'][1] = (apex + top) / 2
    lintel['size'][1] = top - apex
    points = arch_profile(gate['opening'], ARCH)
    for (x0, y0), (x1, y1) in zip(points, points[1:]):
        # Conservative AABBs overlap the curved soffit by at most 10 cm.
        bottom = min(y0, y1)
        blocks.append(dict(name=PROXY_NAME,
                           center=[gate['x'] + (x0 + x1) / 2, (bottom + apex) / 2, gate['z']],
                           size=[x1 - x0, apex - bottom, lintel['size'][2]],
                           color=list(lintel['color']), phaseable=lintel['phaseable'], render=False))
    return index, lintel


def lintel_geometry(block):
    """Closed spandrel with front, back and soffit; game-space world coordinates."""
    x, y, z = block['center']
    width, height, depth = block['size']
    top = y + height / 2
    points = arch_profile(width, block['arch'])
    vertices = []
    for dx, bottom in points:
        vertices.extend([(x + dx, bottom, z - depth / 2),
                         (x + dx, top, z - depth / 2),
                         (x + dx, bottom, z + depth / 2),
                         (x + dx, top, z + depth / 2)])
    faces = [(0, 1, 3, 2)]
    for i in range(len(points) - 1):
        a, b = i * 4, (i + 1) * 4
        faces.extend([(a, b, b + 1, a + 1), (a + 2, a + 3, b + 3, b + 2),
                      (a, a + 2, b + 2, b), (a + 1, b + 1, b + 3, a + 3)])
    a = (len(points) - 1) * 4
    faces.append((a, a + 2, a + 3, a + 1))
    return vertices, faces


def moulding_points(width, arch, z=-.26):
    return [(-width / 2, 0, z)] + [(x, y, z) for x, y in arch_profile(width, arch)] + [(width / 2, 0, z)]
