"""Retarget Ui's independently sampled frog into a seated, visual-only hop."""
import bpy
from mathutils import Matrix, Vector


def apply(rig, base, record, a, b, t, turn):
    samples = record['mount']['samples']
    first, second = samples[a], samples[b]
    lerp = lambda x, y: x + (y - x) * t
    low = min(s['root'][1] for s in samples)
    high = max(s['root'][1] for s in samples)
    # Preserve the source anticipation, ascent, hang and landing times. The
    # original exaggerated 2D jump is adapted to this one-metre-tall mount.
    hop = .42 * (lerp(first['root'][1], second['root'][1]) - low) / (high - low)
    width = 1 + .45 * (lerp(first['scale'][0], second['scale'][0]) - 1)
    height = 1 + .45 * (lerp(first['scale'][1], second['scale'][1]) - 1)
    root = rig.pose.bones['FrogRoot']
    # These bones' local Y axis is armature Z (vertical).
    root.scale = Vector((width, height, width))
    for name, angle in first['rotations'].items():
        if name == 'Mount':
            continue
        value = -lerp(angle, second['rotations'][name]) * .55
        limit = .18 if name == 'Frog.Head' else .5
        turn(name, max(-limit, min(limit, value)))

    mount = rig.pose.bones['Mount']
    mount.location = base['Mount'][1].copy()
    bpy.context.view_layer.update()
    frog = bpy.data.objects['Frog'].evaluated_get(bpy.context.evaluated_depsgraph_get())
    floor = min((frog.matrix_world @ v.co).z for v in frog.data.vertices)
    # Ground the deformed feet first, then lift the whole frog. This retains a
    # real airborne gap instead of flattening the hop with floor correction.
    lift = .008 - floor + hop
    mount.location += mount.bone.matrix_local.to_3x3().inverted() @ Vector((0, 0, lift))

    hips = rig.pose.bones['Hips']
    seat = rig.pose.bones['Hips'].bone.head_local.z + base['Hips'][1].y
    origin = root.bone.head_local.z + base['FrogRoot'][1].y
    relative = [record['samples'][i]['root'][1] - samples[i]['root'][1] for i in range(len(samples))]
    spring = max(-.025, min(.045, .08 * (lerp(relative[a], relative[b]) - relative[0])))
    hips.location = base['Hips'][1] + hips.bone.matrix_local.to_3x3().inverted() @ Vector(
        (0, 0, lift + (seat - origin) * (height - 1) + spring))
    # Keep the authored seated legs; the rider absorbs the frog's landing rather
    # than performing the generic humanoid walking stride independently.
    for name in ['Hips', 'Thigh.L', 'Thigh.R', 'Shin.L', 'Shin.R', 'Foot.L', 'Foot.R']:
        angle = -lerp(record['samples'][a]['rotations'][name], record['samples'][b]['rotations'][name])
        limit = .16 if name == 'Hips' else .22
        turn(name, max(-limit, min(limit, angle * .3)))
    return hop


def turn_from_base(rig, base, name, angle, rotation):
    bone = rig.pose.bones[name]
    rest = bone.bone.matrix_local.to_3x3()
    rotation(bone, base[name][0] @ (rest.inverted() @ Matrix.Rotation(angle, 3, 'X') @ rest).to_quaternion())
