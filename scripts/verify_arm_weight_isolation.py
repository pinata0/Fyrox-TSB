"""Independent geometry, motion, skin-domain and FBX regression checks.

Blender --background --python-exit-code 1 --python this_file [-- slugs]
"""
import bpy, hashlib, json, math, os, sys
import numpy as np
from pathlib import Path
from mathutils import Quaternion

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/arm-isolation'
ASSETS = Path(os.environ.get('TSB_CHARACTER_ASSET_DIR', str(OUT / 'characters')))
sys.path.insert(0, str(ROOT / 'scripts'))
from arm_weight_isolation import arm_bones


def array_hash(values):
    return hashlib.sha256(np.asarray(values).tobytes()).hexdigest()


def snapshot():
    rig = bpy.data.objects['TSB_Rig']
    meshes = {}
    for obj in rig.children:
        if obj.type != 'MESH':
            continue
        mesh = obj.data
        meshes[obj.name] = dict(
            positions=array_hash([v.co[:] for v in mesh.vertices]),
            topology=array_hash([tuple(p.vertices) for p in mesh.polygons] if all(len(p.vertices)==3 for p in mesh.polygons) else [i for p in mesh.polygons for i in (*p.vertices, -1)]),
            uv=array_hash([v.uv[:] for v in mesh.uv_layers.active.data]) if mesh.uv_layers.active else None,
            shapes={k.name: array_hash([v.co[:] for v in k.data]) for k in mesh.shape_keys.key_blocks} if mesh.shape_keys else {},
            weights=[{obj.vertex_groups[g.group].name: g.weight for g in v.groups} for v in mesh.vertices],
        )
    bones = {b.name: (b.parent.name if b.parent else None, list(b.head_local), list(b.tail_local), [list(r) for r in b.matrix_local]) for b in rig.data.bones}
    curves = {}
    for layer in rig.animation_data.action.layers:
        for strip in layer.strips:
            for bag in strip.channelbags:
                for c in bag.fcurves:
                    curves[(c.data_path, c.array_index)] = array_hash([list(k.co) + list(k.handle_left) + list(k.handle_right) for k in c.keyframe_points])
    return meshes, bones, curves


def positions(obj):
    bpy.context.view_layer.update()
    evaluated = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
    return np.array([v.co[:] for v in evaluated.data.vertices])


def stress(slug):
    rig = bpy.data.objects['TSB_Rig']
    body = bpy.data.objects[f'{slug}_SkinnedMesh']
    rig.animation_data_clear()
    for p in rig.pose.bones:
        p.matrix_basis.identity()
        p.rotation_mode = 'QUATERNION'
    for key in body.data.shape_keys.key_blocks:
        key.value = 0
    body.animation_data_clear()
    body.data.shape_keys.animation_data_clear()
    hand = bpy.data.objects[f'{slug}_RefinedHand.L']
    body_before, hand_before = positions(body), positions(hand)
    coat = bpy.data.objects.get(f'{slug}_RestoredCoat')
    coat_before = positions(coat) if coat else None
    for side, sign in [('L', -1), ('R', 1)]:
        rig.pose.bones[f'UpperArm.{side}'].rotation_quaternion = Quaternion((0, 1, 0), sign * math.pi / 2)
        rig.pose.bones[f'Forearm.{side}'].rotation_quaternion = Quaternion((1, 0, 0), 1.1)
        rig.pose.bones[f'Hand.{side}'].rotation_quaternion = Quaternion((0, 0, 1), .8)
    displacement = np.linalg.norm(positions(body) - body_before, axis=1)
    hand_motion = float(np.max(np.linalg.norm(positions(hand) - hand_before, axis=1)))
    result = dict(body_max_arm_displacement_m=float(displacement.max()), hand_motion_m=hand_motion)
    if coat:
        excluded = arm_bones(rig)
        detached = [v.index for v in coat.data.vertices if not any(coat.vertex_groups[g.group].name in excluded and g.weight > 0 for g in v.groups)]
        delta = np.linalg.norm(positions(coat) - coat_before, axis=1)
        result['coat_detached_panel_vertices'] = len(detached)
        result['coat_panel_max_arm_displacement_m'] = float(delta[detached].max()) if detached else None
        result['coat_sleeve_motion_m'] = float(delta.max())
    return result


def main():
    records = json.loads((ROOT / 'assets/characters/manifest.json').read_text(encoding='utf-8'))
    selected = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
    reports = []
    for record in records:
        slug = record['slug']
        if selected and slug not in selected:
            continue
        baseline = Path(os.environ.get('TSB_ARM_BASELINE_DIR', str(OUT / 'before'))) / f'{slug}.blend'
        if not baseline.is_file():
            baseline = OUT / 'before' / f'{slug}.blend'
        bpy.ops.wm.open_mainfile(filepath=str(baseline))
        before = snapshot()
        baseline = stress(slug)
        bpy.ops.wm.open_mainfile(filepath=str(ASSETS / f'{slug}.blend'))
        after = snapshot()
        assert before[1:] == after[1:], (slug, 'bones or motion keys changed')
        assert before[0].keys() == after[0].keys(), (slug, 'mesh set changed')
        for name, old in before[0].items():
            new = after[0][name]
            assert {k: v for k, v in old.items() if k != 'weights'} == {k: v for k, v in new.items() if k != 'weights'}, (slug, name, 'geometry/UV/morph changed')
            if not name.endswith(('_SkinnedMesh', '_RestoredCoat')):
                assert old['weights'] == new['weights'], (slug, name, 'authored arm/companion weights changed')
            assert all(0 < len(row) <= 4 and abs(sum(row.values()) - 1) < 1e-5 for row in new['weights']), (slug, name, 'invalid weights')
        rig = bpy.data.objects['TSB_Rig']
        body = bpy.data.objects[f'{slug}_SkinnedMesh']
        excluded = arm_bones(rig)
        assert all(not excluded.intersection(row) for row in after[0][body.name]['weights']), (slug, 'body still bound to arm')
        motion_samples = 0
        details = json.loads((ASSETS / f'{slug}-rig.json').read_text())
        rest = np.array([v.co[:] for v in body.data.vertices])
        edges = np.array([e.vertices[:] for e in body.data.edges])
        lengths = np.linalg.norm(rest[edges[:, 0]] - rest[edges[:, 1]], axis=1)
        valid = lengths > .006
        max_strain_p99 = 0.
        for clip, (start, end) in details['clips'].items():
            for frame in np.linspace(start, end, 5):
                bpy.context.scene.frame_set(int(frame), subframe=float(frame % 1))
                posed = positions(body)
                assert np.isfinite(posed).all(), (slug, clip, 'non-finite pose')
                ratios = np.linalg.norm(posed[edges[:, 0]] - posed[edges[:, 1]], axis=1)[valid] / lengths[valid]
                strain = float(np.quantile(ratios, .99))
                assert strain < 3.2, (slug, clip, 'skin strain', strain)
                max_strain_p99 = max(max_strain_p99, strain)
                motion_samples += 1
        corrected = stress(slug)
        assert corrected['body_max_arm_displacement_m'] < 1e-6, (slug, corrected)
        assert corrected['hand_motion_m'] > .1, (slug, 'arms stopped moving')
        if 'coat_panel_max_arm_displacement_m' in corrected:
            assert corrected['coat_detached_panel_vertices'] > 200
            assert corrected['coat_panel_max_arm_displacement_m'] < 1e-6
            assert corrected['coat_sleeve_motion_m'] > .1
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.import_scene.fbx(filepath=str(ASSETS / f'{slug}.fbx'))
        imported = next(o for o in bpy.data.objects if o.type=='MESH' and o.name == f'{slug}_SkinnedMesh')
        imported_rig = next(o for o in bpy.data.objects if o.type=='ARMATURE')
        bad = arm_bones(imported_rig)
        assert all(not any(imported.vertex_groups[g.group].name in bad and g.weight > 0 for g in v.groups) for v in imported.data.vertices), (slug, 'FBX arm leakage')
        assert len(imported.data.shape_keys.key_blocks) == 24
        assert len(imported_rig.data.bones) == len(after[1])
        assert max(a.frame_range[1] - a.frame_range[0] for a in bpy.data.actions) >= 527
        for im in bpy.data.images:
            if im.source == 'FILE':
                assert Path(bpy.path.abspath(im.filepath)).is_file(), (slug, im.filepath)
        reports.append(dict(slug=slug, passed=True, before=baseline, after=corrected,
                            geometry_uv_morphs_preserved=True, bones_and_motion_keys_preserved=True,
                            authored_arms_preserved=True, motion_samples=motion_samples,
                            max_edge_strain_p99=max_strain_p99, fbx_roundtrip=True,
                            sha256={suffix: hashlib.sha256((ASSETS / f'{slug}{suffix}').read_bytes()).hexdigest()
                                    for suffix in ('.blend', '.fbx', '-rig.json')}))
        (OUT / 'verification.json').write_text(json.dumps(dict(passed=len(reports)==len(records), characters=reports), indent=2))
        print('ARM_ISOLATION_VERIFIED', slug, json.dumps(corrected), flush=True)


if __name__ == '__main__':
    main()
