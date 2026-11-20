"""Move Id's raised white lashes with the lid instead of drawing dark lashes.

The white tufts are part of the continuous head mesh, in front of the painted
eye. Depth separates them from the eyelid skin; the upper envelope excludes
the overhanging fringe. All coordinates are in the authored character basis.
"""
import numpy as np
from blink_surfaces import smooth


def lash_weight(points, face, side):
    x, y, z = points.T
    sign = -1 if side == 'L' else 1
    outward = sign * (x - face['center_x'])
    # The source is asymmetric: use each tuft's actual front/back boundary.
    back = np.interp(outward, [.055, .08, .11, .14, .17, .20, .245],
                     [-.197, -.194, -.181, -.168, -.150, -.127, -.092]
                     if side == 'L' else
                     [-.183, -.178, -.171, -.160, -.145, -.120, -.085])
    top = np.interp(outward, [.055, .09, .14, .19, .22, .25],
                    [1.313, 1.324, 1.328, 1.323, 1.332, 1.340])
    weight = smooth((back - y - .008) / .014)
    inner = .055 if side=='L' else .105
    weight *= smooth((outward - inner) / .018) * (1-smooth((outward-.235)/.020))
    weight *= smooth((z-1.286)/.010) * (1-smooth((z-top)/.006))
    return weight


def lid_height(x, face, side, closure):
    sign = -1 if side == 'L' else 1
    outward = sign*(x-face['center_x'])
    arc = np.clip((outward-.145)/.105, -1, 1)**2
    return (1.307+.004*arc)*(1-closure) + (face['eye_y']-.010+.010*arc)*closure


def eye_coverage(points, face, side):
    x, y, z = points.T
    sign = -1 if side == 'L' else 1
    outward = sign*(x-face['center_x'])
    coverage = smooth((outward-.042)/.012) * (1-smooth((outward-.214)/.018))
    # The painted eyes end at 1.250, well above the generic face landmark's
    # lower bound. Limit the underlay to the eyes to retain the cheek gradient.
    coverage *= smooth((z-1.240)/.009) * (1-smooth((z-1.339)/.006))
    # Keep the hair sheets and original sculpted white lash colour intact.
    back = np.interp(outward, [.055,.08,.11,.14,.17,.20,.245],
                     [-.197,-.194,-.181,-.168,-.150,-.127,-.092]
                     if side == 'L' else
                     [-.183,-.178,-.171,-.160,-.145,-.120,-.085])
    coverage *= smooth((y-back+.075)/.018)
    coverage *= 1-smooth((y-back-.025)/.030)
    # The tuft joins the face through a tapered wall. Paint that attachment as
    # skin while leaving only the fully moving white strip visible, so lowering
    # it cannot stretch its grey upper border into a second eyebrow.
    weight=lash_weight(points,face,side)
    coverage *= 1-smooth((weight-.80)/.19)
    if side=='R':
        # The inner end disappears beneath the central fringe. That fringe is
        # deeper than the tuft and must neither move nor receive eyelid paint.
        hair=smooth((.122-outward)/.018)*smooth((back-y-.012)/.012)
        coverage *= 1-hair
    return coverage


def upper_shadow_coverage(points, face, side, colors):
    """Erase the pink open-lid crease when the white lash has moved down."""
    x,y,z=points.T
    sign=-1 if side=='L' else 1
    outward=sign*(x-face['center_x'])
    back=np.interp(outward,[.055,.08,.11,.14,.17,.20,.245],
                   [-.197,-.194,-.181,-.168,-.150,-.127,-.092]
                   if side=='L' else
                   [-.183,-.178,-.171,-.160,-.145,-.120,-.085])
    coverage=smooth((outward-.042)/.012)*(1-smooth((outward-.214)/.018))
    coverage*=smooth((z-1.320)/.009)*(1-smooth((z-1.360)/.010))
    coverage*=smooth((y-back+.042)/.014)*(1-smooth((y-back-.025)/.030))
    # Only warm facial paint belongs to this crease. The white fringe can
    # overlap it at the same height and must retain its original colour.
    coverage*=smooth((colors[:,0]-colors[:,1]-.015)/.025)
    coverage*=smooth((colors[:,0]-colors[:,2]-.010)/.020)
    return coverage


def rebuild_white_lashes(body, face):
    mesh = body.data
    basis = np.array([v.co[:] for v in mesh.shape_keys.key_blocks[0].data])
    stats = {}
    for side in ['L', 'R']:
        weight = lash_weight(basis, face, side)
        key = mesh.shape_keys.key_blocks['eyeBlink_'+side]
        target = np.array([v.co[:] for v in key.data])
        # Preserve the white tuft's depth and thickness, including its tips.
        target[:, 1] = target[:, 1]*(1-weight)+basis[:, 1]*weight
        down = lid_height(basis[:, 0], face, side, 1)-lid_height(basis[:, 0], face, side, 0)
        brow = mesh.shape_keys.key_blocks['browDown_'+side]
        brow_delta = np.array([v.co[:] for v in brow.data])-basis
        # Runtime adds 0.75 * browDown during a blink. Compensate on the tuft
        # without modifying the independently authored eyebrow expression.
        target[:, 2] = basis[:, 2]+(down-.75*brow_delta[:, 2])*weight
        key.data.foreach_set('co', target.astype(np.float32).ravel())
        normal_weight = mesh.attributes.get('blink_weight_'+side)
        if normal_weight:
            values = np.array([v.value for v in normal_weight.data])*(1-weight)
            normal_weight.data.foreach_set('value', values.astype(np.float32))
        stats[side] = dict(vertices=int((weight>.01).sum()),
                           full_weight_vertices=int((weight>.99).sum()),
                           max_drop=float((-down*weight).max()))
    mesh.update()
    return stats
