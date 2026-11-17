"""Bake Chloe's two palms underneath Sebastian's cheeks, from the original art.

Run in Blender with --background --factory-startup --python-exit-code 1 --python.
Builds an isolated candidate from a once-only backup of the current assets.
"""
import bpy
import hashlib
import json
import math
import shutil
import sys
from pathlib import Path

import numpy as np
from mathutils import Matrix, Quaternion, Vector
from mathutils.bvhtree import BVHTree

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx, resolve_character_textures
from pose_id_rider import arm_ik, world_rotation
import resize_companions as review

WORK = ROOT/'artifacts/chloe-carry'
BEFORE = WORK/'before'
OUT = WORK/'candidate'
ASSETS = ROOT/'assets/characters'
UPPER_LENGTH = .67
FORE_LENGTH = .53
HEAD_FORWARD = .40
FILES = ['chloe.blend', 'chloe.fbx', 'chloe.morphs.json', 'chloe-hands.json']


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def smooth(value):
    value = max(0., min(1., value))
    return value*value*(3.-2.*value)


def shift_head(co):
    result = co.copy()
    blend = max(smooth((co.z-1.48)/.24), smooth((abs(co.x)-.65)/.15)*smooth((co.z-1.10)/.18))
    result.y -= HEAD_FORWARD*blend
    return result


def stretch(co, origin, direction, scale, offset):
    return co + direction*((co-origin).dot(direction)*(scale-1)) + offset


def palm_rotation(profile, side):
    # Rest-space dorsal points away from the palm. Both palms face upward,
    # with the fingers turned inward beneath the lower edge of each cheek.
    dorsal = Vector((0., .25, -1.)).normalized()
    forward = Vector((1. if side == 'L' else -1., .20, 0.))
    forward = (forward-dorsal*forward.dot(dorsal)).normalized()
    across = dorsal.cross(forward).normalized()
    original = Matrix((profile['forward'], profile['across'], profile['dorsal'])).transposed()
    if original.determinant() < 0:
        across.negate()
    desired = Matrix((forward, across, dorsal)).transposed()
    return (desired @ original.inverted()).to_quaternion()


def build():
    BEFORE.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    for name in FILES:
        if not (BEFORE/name).exists():
            shutil.copy2(ASSETS/name, BEFORE/name)
    bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'chloe.blend'))
    scene = bpy.context.scene
    rig = bpy.data.objects['TSB_Rig']
    body = bpy.data.objects['chloe_SkinnedMesh']
    bear = bpy.data.objects['Sebastian']
    meshes = [o for o in scene.objects if o.type == 'MESH']
    resolve_character_textures(meshes)
    for obj in meshes:
        for mat in obj.data.materials:
            if not mat or not mat.use_nodes:
                continue
            for node in mat.node_tree.nodes:
                if node.type == 'TEX_IMAGE' and node.image:
                    image = node.image
                    path = Path(bpy.path.abspath(image.filepath))
                    assert path.is_file(), path
                    if not (OUT/path.name).exists():
                        shutil.copy2(path, OUT/path.name)
                    image.filepath = str(OUT/path.name)
    scene.frame_set(0)
    profiles = json.loads((BEFORE/'chloe-hands.json').read_text())
    # The prior sleeve cleanup isolated all arm weights from the body and face.
    arm_groups = {g.index for g in body.vertex_groups if g.name.startswith(('UpperArm.', 'Forearm.', 'Hand.'))}
    assert not any(w.group in arm_groups and w.weight > 1e-6 for v in body.data.vertices for w in v.groups)
    old_rest = {b.name: b.matrix_local.copy() for b in rig.data.bones}
    frames = []
    for frame in range(529):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        frames.append({b.name: b.matrix.copy() for b in rig.pose.bones})
    # Bring the bear's head over the crown; the torso and long arms remain behind.
    for v in bear.data.vertices:
        v.co = shift_head(v.co)
    bear.data.update()
    offsets = {}
    for profile in profiles:
        side = profile['side']
        s, e, w = [rig.data.bones[f'{part}.{side}'].head_local.copy() for part in ['UpperArm','Forearm','Hand']]
        a, b = (e-s).length, (w-e).length
        da, db = (e-s).normalized(), (w-e).normalized()
        new_e = s + da*UPPER_LENGTH
        new_w = new_e + db*FORE_LENGTH
        offsets[side] = dict(elbow=new_e-e, wrist=new_w-w)
        # Extend the existing closed sleeves along their actual limb axes,
        # preserving the hands, face and torso rather than stretching them.
        for obj in meshes:
            if obj.name in [f'chloe_RefinedSleeve.{side}', f'chloe_RefinedViewSleeve.{side}']:
                for v in obj.data.vertices:
                    weights = {obj.vertex_groups[g.group].name: g.weight for g in v.groups}
                    upper = stretch(v.co, s, da, UPPER_LENGTH/a, Vector())
                    fore = stretch(v.co, e, db, FORE_LENGTH/b, new_e-e)
                    u = weights.get(f'UpperArm.{side}', 0.)
                    f = weights.get(f'Forearm.{side}', 0.)
                    v.co = (upper*u + fore*f)/max(u+f, 1e-8)
                obj.data.update()
            elif obj.name == f'chloe_RefinedHand.{side}':
                for v in obj.data.vertices:
                    v.co += new_w-w
                obj.data.update()
        profile['wrist'] = list(new_w)
        profile['contact'] = list(Vector(profile['contact']) + new_w-w)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode='EDIT')
    for bone in rig.data.edit_bones:
        if bone.name.startswith('Sebastian'):
            delta = shift_head(bone.head)-bone.head
            bone.head += delta
            bone.tail += delta
        for side, offset in offsets.items():
            if bone.name == f'Forearm.{side}':
                delta = offset['elbow']
            elif bone.name.endswith('.'+side) and bone.name.startswith(('Hand.','PalmGrip.','Index','Middle','Ring','Pinky','Thumb')):
                delta = offset['wrist']
            else:
                continue
            bone.head += delta
            bone.tail += delta
    bpy.ops.object.mode_set(mode='OBJECT')
    # Match the long yellow sleeves and white cuffs visible in the original art.
    for obj in meshes:
        if '_RefinedSleeve.' not in obj.name and '_RefinedViewSleeve.' not in obj.name:
            continue
        for index, old in enumerate(list(obj.data.materials)):
            color = 'f3eee5' if 'trim' in old.name.lower() else 'e4af49'
            mat = old.copy()
            mat.name = f'Color_{color}_ChloeCarrySleeve_{index}'
            rgb = tuple(int(color[i:i+2],16)/255 for i in (0,2,4)) + (1.,)
            mat.diffuse_color = rgb
            if 'meshy_base_texture' in mat:
                del mat['meshy_base_texture']
            bsdf = mat.node_tree.nodes.get('Principled BSDF')
            for link in list(bsdf.inputs['Base Color'].links):
                mat.node_tree.links.remove(link)
            bsdf.inputs['Base Color'].default_value = rgb
            obj.data.materials[index] = mat
    rest = {b.name: b.matrix_local.copy() for b in rig.data.bones}
    # Surface vertices at the undersides of the cheeks provide deforming contact
    # anchors. Their actual evaluated positions are sampled before solving arms.
    points = np.array([v.co[:] for v in bear.data.vertices])
    bear.data.calc_loop_triangles()
    triangles = [tuple(t.vertices) for t in bear.data.loop_triangles]
    bvh = BVHTree.FromPolygons([Vector(p) for p in points], triangles, all_triangles=True)
    anchors = {}
    for side, sign in [('L',-1),('R',1)]:
        hits = []
        for x in np.linspace(.42,.64,12):
            for y in np.linspace(-.26,-.02,13):
                hit, normal, index, distance = bvh.ray_cast(Vector((sign*x,y,1.52)), Vector((0,0,1)), .45)
                if hit is not None and normal.z < -.25 and hit.z > 1.60:
                    score = (hit-Vector((sign*.54,-.15,1.71))).length
                    hits.append((score,hit,index))
        assert hits, side
        _, hit, index = min(hits,key=lambda row:row[0])
        tri = triangles[index]
        a, b, c = points[list(tri)]
        weights = np.linalg.lstsq(np.column_stack((b-a,c-a)), np.array(hit)-a,rcond=None)[0]
        anchors[side] = dict(vertices=tri, weights=[float(1-weights.sum()),*weights.tolist()], rest=list(hit))
    joints = [f'{part}.{s}' for s in ['L','R'] for part in ['UpperArm','Forearm','Hand']]
    errors = []
    for frame, saved in enumerate(frames):
        scene.frame_set(frame)
        # Existing torso, face, legs and independent bear animation are retained.
        for name in joints:
            p = rig.pose.bones[name]
            p.rotation_mode = 'QUATERNION'
            p.rotation_quaternion = Quaternion()
            p.location = Vector()
            p.scale = Vector((1,1,1))
        bpy.context.view_layer.update()
        evaluated = bear.evaluated_get(bpy.context.evaluated_depsgraph_get())
        targets = {s: sum((evaluated.data.vertices[i].co*w for i,w in zip(a['vertices'],a['weights'])),Vector()) for s,a in anchors.items()}
        bear_delta = rig.pose.bones['Sebastian.Head'].matrix @ rest['Sebastian.Head'].inverted()
        for profile in profiles:
            side = profile['side']
            rotation = bear_delta.to_quaternion() @ palm_rotation(profile, side)
            # The contact point is on the palm skin; leave a tiny cushion overlap.
            target = targets[side] + bear_delta.to_3x3() @ Vector((0,0,-.004))
            wrist = target - rotation @ (Vector(profile['contact'])-Vector(profile['wrist']))
            arm_ik(rig, side, wrist)
            world_rotation(rig, f'Hand.{side}', rotation @ rest[f'Hand.{side}'].to_quaternion())
            hand_delta = rig.pose.bones[f'Hand.{side}'].matrix @ rest[f'Hand.{side}'].inverted()
            contact = hand_delta @ Vector(profile['contact'])
            errors.append((contact-target).length)
        for name in joints:
            p = rig.pose.bones[name]
            p.keyframe_insert(data_path='rotation_quaternion', frame=frame, group=name)
            p.keyframe_insert(data_path='location', frame=frame, group=name)
            p.keyframe_insert(data_path='scale', frame=frame, group=name)
    scene.frame_set(0)
    bpy.context.view_layer.update()
    report = dict(source_sha256={n:digest(BEFORE/n) for n in FILES},
                  head_forward_m=HEAD_FORWARD, upper_arm_length_m=UPPER_LENGTH,
                  forearm_length_m=FORE_LENGTH, cheek_anchors=anchors,
                  maximum_palm_contact_error_m=max(errors), clips=json.loads(scene['clips']))
    scene['chloe_carry_pose'] = json.dumps(report)
    scene['hand_rig'] = json.dumps(profiles)
    (OUT/'chloe-hands.json').write_text(json.dumps(profiles, indent=2))
    for image in bpy.data.images:
        if image.source == 'FILE' and image.filepath and Path(bpy.path.abspath(image.filepath)).is_file():
            image.filepath = '//' + Path(bpy.path.abspath(image.filepath)).name
    bpy.context.preferences.filepaths.save_version = 0
    bpy.data.use_autopack = False
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'chloe.blend'), relative_remap=False)
    export_fbx(body, rig, OUT/'chloe.fbx')
    (WORK/'pose.json').write_text(json.dumps(report, indent=2))
    if '--skip-render' not in sys.argv:
        review.WORK = WORK
        review.render(scene, 'chloe', 'carry', ('front','side','three-quarter'))
    print('CHLOE_CARRY_BUILT', max(errors), flush=True)


def publish():
    verification = json.loads((WORK/'verification.json').read_text())
    report = json.loads((WORK/'pose.json').read_text())
    assert verification['passed']
    for name, expected in verification['sha256'].items():
        assert digest(OUT/name) == expected, name
        assert digest(ASSETS/name) in [report['source_sha256'][name], expected], f'{name} changed independently'
    for name in FILES:
        shutil.copy2(OUT/name, ASSETS/name)
    records = json.loads((ASSETS/'manifest.json').read_text(encoding='utf-8'))
    row = next(r for r in records if r['slug'] == 'chloe')
    row['carry_pose'] = report
    for key in ['blend','model']:
        row['sha256'][key] = digest(ROOT/row[key])
    (ASSETS/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('CHLOE_CARRY_PUBLISHED', flush=True)


if __name__ == '__main__':
    publish() if '--publish' in sys.argv else build()
