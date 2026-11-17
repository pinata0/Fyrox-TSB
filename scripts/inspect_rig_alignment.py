"""Render rest-pose joint markers over front/profile views for landmark fitting."""
import argparse
import json
import sys
from pathlib import Path
import bpy
import numpy as np
from mathutils import Vector

parser = argparse.ArgumentParser()
parser.add_argument('--blend', type=Path, required=True)
parser.add_argument('--slug', required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
args.output.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(args.blend.resolve()))
rig = bpy.data.objects['TSB_Rig']
rig.data.pose_position = 'REST'
scene = bpy.context.scene
scene.frame_set(0)
mesh = bpy.data.objects[args.slug + '_SkinnedMesh']
points = np.array([(mesh.matrix_world @ v.co)[:] for v in mesh.data.vertices])
target = Vector((0, 0, (points[:, 2].min() + points[:, 2].max()) / 2))
names = ['UpperArm.R', 'Forearm.R', 'Hand.R', 'Hips', 'Torso', 'Head', 'Hair']
colors = [(1, 0, 0, 1), (0, 1, 0, 1), (0, .4, 1, 1), (1, 1, 0, 1), (1, .4, 0, 1), (1, 0, 1, 1), (0, 1, 1, 1)]
markers = []
stats = {'projection': {'resolution': 768, 'orthographic_scale': scene.camera.data.ortho_scale,
                         'target_z': target.z, 'front_center_x': 384, 'front_center_y': 384}}
for name, color in zip(names, colors):
    point = rig.matrix_world @ rig.data.bones[name].head_local
    selection = points[np.linalg.norm(points[:, [0, 2]] - np.array([point.x, point.z]), axis=1) < .06]
    stats[name] = {'joint': list(point), 'surface_y_at_joint_xz': np.percentile(selection[:, 1], [0, 20, 50, 80, 100]).tolist() if len(selection) else []}
    mat = bpy.data.materials.new('Marker_' + name)
    mat.use_nodes = True
    mat.node_tree.nodes.clear()
    emission = mat.node_tree.nodes.new('ShaderNodeEmission')
    emission.inputs['Color'].default_value = color
    output = mat.node_tree.nodes.new('ShaderNodeOutputMaterial')
    mat.node_tree.links.new(emission.outputs[0], output.inputs['Surface'])
    bpy.ops.mesh.primitive_uv_sphere_add(segments=12, ring_count=8, radius=.025, location=point)
    marker = bpy.context.object
    marker.data.materials.append(mat)
    markers.append((marker, point.copy()))
scene.render.resolution_x = scene.render.resolution_y = 768
scene.cycles.samples = 12
for view in ['front', 'profile']:
    for marker, point in markers:
        marker.location = (point.x, -3, point.z) if view == 'front' else (3, point.y, point.z)
    scene.camera.location = (0, -5, target.z) if view == 'front' else (5, 0, target.z)
    scene.camera.rotation_euler = (target - scene.camera.location).to_track_quat('-Z', 'Y').to_euler()
    scene.render.filepath = str((args.output / (view + '.png')).resolve())
    bpy.ops.render.render(write_still=True)
(args.output / 'alignment.json').write_text(json.dumps(stats, indent=2), encoding='utf-8')
