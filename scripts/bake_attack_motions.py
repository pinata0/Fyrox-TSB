"""Retarget normal Attack1_1 into frames 156..180; retain all other clips and meshes.
Run: blender --background --python-exit-code 1 --python scripts/bake_attack_motions.py
"""
import hashlib, json, math, shutil, sys
from pathlib import Path
import bpy
from mathutils import Matrix, Vector
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
OUT=ROOT/'artifacts/basic-attacks'
BACKUP=OUT/'before/assets/characters'; BACKUP.mkdir(parents=True,exist_ok=True)
DATA=json.loads((ROOT/'assets/characters/attack/motions.json').read_text(encoding='utf-8'))

def curves(rig):
    return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves

def mesh_hash(mesh):
    h=hashlib.sha256()
    for v in mesh.data.vertices:h.update(repr((tuple(v.co),[(g.group,g.weight) for g in v.groups])).encode())
    for p in mesh.data.polygons:h.update(repr((tuple(p.vertices),p.material_index)).encode())
    return h.hexdigest()

def rotation(p,q):
    if p.rotation_mode=='QUATERNION':
        if p.rotation_quaternion.dot(q)<0:q=-q
        p.rotation_quaternion=q
    else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)

def smooth(t):
    t=max(0,min(1,t));return t*t*(3-2*t)

reports=[];checkpoints=[]
only=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
for record in DATA:
    slug=record['slug']
    if only and slug not in only:continue
    blend=ROOT/f'assets/characters/{slug}.blend';fbx=blend.with_suffix('.fbx')
    for path in (blend,fbx):
        if not (BACKUP/path.name).exists():shutil.copy2(path,BACKUP/path.name)
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects[f'{slug}_SkinnedMesh'];scene=bpy.context.scene
    def other_keys():
        return [(c.data_path,c.array_index,[tuple(p.co) for p in c.keyframe_points if not 156<=p.co.x<=180]) for c in curves(rig)]
    before=other_keys();geometry=mesh_hash(mesh)
    scene.frame_set(0)
    base={p.name:(p.rotation_quaternion.copy() if p.rotation_mode=='QUATERNION' else p.rotation_euler.to_quaternion(),p.location.copy(),p.scale.copy()) for p in rig.pose.bones}
    floors=[]
    for frame in range(156,181):
        phase=(frame-156)/24;scene.frame_set(frame)
        for p in rig.pose.bones:
            q,loc,scale=base[p.name];rotation(p,q);p.location=loc;p.scale=scale
        at=phase*(len(record['samples'])-1);a=int(at);b=min(a+1,len(record['samples'])-1);t=at-a
        first=record['samples'][a];second=record['samples'][b]
        # A short blend handles flat sprites whose first/last attack pose differs from Idle.
        weight=smooth(phase/.08)*smooth((1-phase)/.14)
        for name,angle in first['rotations'].items():
            value=-(angle+(second['rotations'][name]-angle)*t)*weight
            # Compress sprite-only full turns, keeping a solid wrist/elbow within its range.
            limit=1.8 if name.startswith(('Forearm','Hand')) else 2.0
            if name=='Hips':limit=.24
            elif name in ('Torso','Head'):limit=.55
            elif name.startswith(('Thigh','Shin','Foot')):limit=.5
            value=max(-limit,min(limit,value))
            p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3()
            delta=rest.inverted()@Matrix.Rotation(value,3,'X')@rest
            rotation(p,base[name][0]@delta.to_quaternion())
        root=[(first['root'][j]+(second['root'][j]-first['root'][j])*t)*weight for j in range(2)]
        hips=rig.pose.bones['Hips'];rest=hips.bone.matrix_local.to_3x3()
        hips.location=base['Hips'][1]+rest.inverted()@Vector((0,-.28*math.tanh(root[0]),max(-.12,min(.18,root[1]*.45))))
        bpy.context.view_layer.update()
        evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        floor=min((evaluated.matrix_world@v.co).z for v in evaluated.data.vertices)
        if floor<.012 and 0<phase<1:hips.location+=rest.inverted()@Vector((0,0,.012-floor))
        floors.append(round(floor,5))
        for p in rig.pose.bones:
            p.keyframe_insert(data_path='rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=p.name)
            p.keyframe_insert(data_path='location',frame=frame,group=p.name)
    # Linear baked keys avoid Bezier overshoot around quick reversals.
    for c in curves(rig):
        for key in c.keyframe_points:
            if 156<=key.co.x<=180:key.interpolation='LINEAR'
    assert other_keys()==before,(slug,'unrelated keys changed')
    assert mesh_hash(mesh)==geometry,(slug,'mesh/weights changed')
    poses=[]
    for phase in [0,.2,.4,.6,.8,1]:
        frame=156+phase*24;scene.frame_set(int(frame),subframe=frame-int(frame));bpy.context.view_layer.update()
        poses.append({name:list(rig.matrix_world@rig.pose.bones[name].head) for name in ['Hips','Head','Hand.L','Hand.R']})
    checkpoints.append({'slug':slug,'poses':poses})
    scene['attack_source']='EPID battle Attack1_1; sampled at 30 Hz; 3D adaptation in frames 156-180'
    scene['attack_source_seconds']=record['duration'];scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(blend));export_fbx(mesh,rig,fbx)
    reports.append({'slug':slug,'source_seconds':record['duration'],'mesh_sha256':geometry,'unrelated_keys_preserved':True,'pre_correction_floors':floors})
    (OUT/'asset-verification.json').write_text(json.dumps(reports,indent=2),encoding='utf-8')
    print('ATTACK_ASSET_OK',slug,flush=True)
(ROOT/'assets/characters/attack/pose-checkpoints.json').write_text(json.dumps(checkpoints),encoding='utf-8')
print('ATTACK_ASSETS_COMPLETE',len(reports),flush=True)
