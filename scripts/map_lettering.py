"""Keep wayfinding symbols visible when using Hungeul's text glyphs."""
import bmesh


def font_body(body):
    # Hungeul contains the lettering but has no U+2191/U+2192 arrow glyphs.
    return body[:-1].rstrip() if body.endswith(('↑', '→')) else body


def add_direction_arrow(mesh, body, size):
    if not body.endswith(('↑', '→')):
        return
    outline = [(-.4, -.085), (.05, -.085), (.05, -.25), (.4, 0),
               (.05, .25), (.05, .085), (-.4, .085)]
    if body.endswith('↑'):
        outline = [(-y, x) for x, y in outline]
    right = max(v.co.x for v in mesh.vertices)
    center_y = (min(v.co.y for v in mesh.vertices) + max(v.co.y for v in mesh.vertices)) / 2
    center_x = right + size * (.3 - min(x for x, y in outline))
    bm = bmesh.new()
    bm.from_mesh(mesh)
    rings = [[bm.verts.new((center_x + x * size, center_y + y * size, z))
              for x, y in outline] for z in [-.008, .008]]
    bm.faces.new(list(reversed(rings[0])))
    bm.faces.new(rings[1])
    for i in range(len(outline)):
        j = (i + 1) % len(outline)
        bm.faces.new([rings[0][i], rings[0][j], rings[1][j], rings[1][i]])
    # Center the combined lettering and symbol on the existing sign origin.
    center_x = (min(v.co.x for v in bm.verts) + max(v.co.x for v in bm.verts)) / 2
    for vertex in bm.verts:
        vertex.co.x -= center_x
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()
