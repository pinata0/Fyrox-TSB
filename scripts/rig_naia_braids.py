"""Add independent three-joint braids and retarget Naia's normal Attack1_1.

The original mesh, UVs, facial shapes, and unrelated body keys are retained.
Run with Blender --background --python-exit-code 1 --python scripts/rig_naia_braids.py.
"""
import bpy, json, math, sys, hashlib
import numpy as np
from pathlib import Path
from mathutils import Vector, Matrix, Quaternion
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
OUT=ROOT/'artifacts/naia-attack';ASSETS=ROOT/'assets/characters'
detail=json.loads((ASSETS/'attack/naia-detail.json').read_text())
record=next(x for x in json.loads((ASSETS/'attack/motions.json').read_text()) if x['slug']=='naia')
blend=ASSETS/'naia.blend'
# Reproducible from the task backup, never accumulate rotations/weights on reruns.
bpy.ops.wm.open_mainfile(filepath=str(OUT/'before/assets/characters/naia.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects['naia_SkinnedMesh']
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
def unrelated():return {(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if not 156<=k.co.x<=180] for c in curves() if 'Braid' not in c.data_path and any(not 156<=k.co.x<=180 for k in c.keyframe_points)}
before=unrelated()
coords=np.array([v.co[:] for v in mesh.data.vertices]);geometry=coords.copy()
original_weights=[[(g.group,g.weight) for g in v.groups] for v in mesh.data.vertices]
scene.frame_set(0)
base={p.name:(p.rotation_quaternion.copy() if p.rotation_mode=='QUATERNION' else p.rotation_euler.to_quaternion(),p.location.copy(),p.scale.copy()) for p in rig.pose.bones}
def rotation(p,q):
    if p.rotation_mode=='QUATERNION':
        if p.rotation_quaternion.dot(q)<0:q=-q
        p.rotation_quaternion=q
    else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)
def smooth(x):x=max(0,min(1,x));return x*x*(3-2*x)
head_groups={mesh.vertex_groups[n].index for n in ['Head','Hair']}
selected={s:[] for s in ['L','R']}
influence={}
for v in mesh.data.vertices:
    x,y,z=v.co
    head_weight=sum(g.weight for g in v.groups if g.group in head_groups)
    factor=smooth((abs(x)-.08)/.07)*smooth((y-.06)/.08)*smooth((head_weight-.30)/.35)
    if factor>1e-7 and z<1.17:
        selected['L' if x<0 else 'R'].append(v.index)
        influence[v.index]=factor
assert all(len(ids)>350 for ids in selected.values()),{s:len(i) for s,i in selected.items()}
bpy.context.view_layer.objects.active=rig;rig.select_set(True);bpy.ops.object.mode_set(mode='EDIT')
centers={}
for side,ids in selected.items():
    points=coords[[i for i in ids if influence[i]>.99]];zs=[1.15,.94,.73,.49];centers[side]=[]
    for z in zs:
        section=points[np.abs(points[:,2]-z)<.065]
        assert len(section)>5,(side,z,len(section))
        centers[side].append(Vector((float(np.median(section[:,0])),float(np.median(section[:,1])),z)))
    for i in range(3):
        bone=rig.data.edit_bones.new(f'Braid.{side}.{i+1:02}')
        bone.head=centers[side][i];bone.tail=centers[side][i+1]
        bone.parent=rig.data.edit_bones['Head' if i==0 else f'Braid.{side}.{i:02}'];bone.use_connect=i>0
bpy.ops.object.mode_set(mode='OBJECT')
for side,ids in selected.items():
    names=['Head']+[f'Braid.{side}.{i+1:02}' for i in range(3)]
    groups=[mesh.vertex_groups.get(n) or mesh.vertex_groups.new(name=n) for n in names]
    # Smoothly blend at each joint; the scalp itself remains fully on Head.
    heights=[1.17,1.00,.82,.60]
    for idx in ids:
        v=mesh.data.vertices[idx];z=v.co.z
        weights=[0.0]*4
        if z>=heights[0]:weights[0]=1
        elif z<=heights[-1]:weights[-1]=1
        else:
            i=next(i for i in range(3) if heights[i]>=z>=heights[i+1])
            t=smooth((heights[i]-z)/(heights[i]-heights[i+1]));weights[i]=1-t;weights[i+1]=t
        factor=influence[idx]
        merged={g.group:g.weight*(1-factor) for g in v.groups}
        for g,w in zip(groups,weights):merged[g.index]=merged.get(g.index,0)+w*factor
        merged=sorted(((g,w) for g,w in merged.items() if w>1e-7),key=lambda pair:pair[1],reverse=True)[:4]
        total=sum(w for _,w in merged)
        for g in list(v.groups):mesh.vertex_groups[g.group].remove([idx])
        for g,w in merged:mesh.vertex_groups[g].add([idx],w/total,'REPLACE')
    for n in names[1:]:rig.pose.bones[n].rotation_mode='QUATERNION'

def sample(samples,phase):
    at=phase*(len(samples)-1);a=int(at);b=min(a+1,len(samples)-1);return samples[a],samples[b],at-a
def aim(name,end,target):
    p=rig.pose.bones[name];current=rig.pose.bones[end].head-p.head
    q=current.rotation_difference(target-p.head)
    p.matrix=Matrix.Translation(p.head)@q.to_matrix().to_4x4()@Matrix.Translation(-p.head)@p.matrix
    bpy.context.view_layer.update()
def solve_arm(side,target):
    upper=f'UpperArm.{side}';fore=f'Forearm.{side}';hand=f'Hand.{side}'
    a=rig.pose.bones[upper].head.copy();b=rig.pose.bones[fore].head.copy();c=rig.pose.bones[hand].head.copy()
    l1=(b-a).length;l2=(c-b).length;direction=target-a
    distance=max(abs(l1-l2)+.001,min(l1+l2-.002,direction.length));direction.normalize();target=a+direction*distance
    pole=Vector((-1 if side=='L' else 1,-.35,-.75));pole-=direction*pole.dot(direction);pole.normalize()
    along=(l1*l1-l2*l2+distance*distance)/(2*distance);height=math.sqrt(max(0,l1*l1-along*along))
    elbow=a+direction*along+pole*height
    aim(upper,fore,elbow);aim(fore,hand,target)

# Bake secondary motion across existing clips so transitions never strand a braid.
for frame in range(529):
    scene.frame_set(frame)
    for side in ['L','R']:
        for i in range(3):
            p=rig.pose.bones[f'Braid.{side}.{i+1:02}']
            # Follow the old low-amplitude secondary track at the new scalp pivot.
            hair=rig.pose.bones['Hair'];e=hair.rotation_quaternion.to_euler() if hair.rotation_mode=='QUATERNION' else hair.rotation_euler
            rotation(p,Quaternion((1,0,0),float(e.x)*(.5 if i==0 else .22)))
            p.keyframe_insert('rotation_quaternion',frame=frame,group=p.name)

floors=[]
for frame in range(156,181):
    phase=(frame-156)/24;scene.frame_set(frame)
    for name,(q,loc,scale) in base.items():
        p=rig.pose.bones[name];rotation(p,q);p.location=loc;p.scale=scale
    first,second,t=sample(record['samples'],phase);weight=smooth(phase/.08)*smooth((1-phase)/.14)
    for name,angle in first['rotations'].items():
        if name.startswith(('UpperArm','Forearm','Hand')):continue
        value=angle+(second['rotations'][name]-angle)*t
        # Sprite IK can invert a foot axis by 180 degrees. Keep planted feet stable.
        if name.startswith('Foot'):value=0
        limit=.22 if name=='Hips' else .38 if name in ['Torso','Head'] else .42
        value=max(-limit,min(limit,-value))*weight
        p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3()
        rotation(p,base[name][0]@(rest.inverted()@Matrix.Rotation(value,3,'X')@rest).to_quaternion())
    root=[(first['root'][j]+(second['root'][j]-first['root'][j])*t)*weight for j in range(2)]
    hips=rig.pose.bones['Hips'];rest=hips.bone.matrix_local.to_3x3()
    hips.location=base['Hips'][1]+rest.inverted()@Vector((0,-.24*root[0],.40*root[1]))
    bpy.context.view_layer.update()
    for index,side in enumerate(['L','R']):
        path=[first['hands'][index][j]+(second['hands'][index][j]-first['hands'][index][j])*t for j in range(3)]
        # Retarget wrist positions, avoiding the original 2D elbow's full-turn discontinuity.
        hand=rig.pose.bones[f'Hand.{side}'];origin=hand.head.copy()
        reach=smooth(phase/.18)*smooth((1-phase)/.20)
        weapon_forward=first['hands'][0][0]+(second['hands'][0][0]-first['hands'][0][0])*t
        cross=smooth(-weapon_forward/.70)
        desired=Vector((-.20,-.12+.32*path[0],1.10+.45*path[1])) if side=='L' else Vector((.14-.27*cross,-.08-.26*cross,1.18-.09*cross+.45*path[1]))
        desired+=Vector((0,-.24*root[0],.40*root[1]))
        target=origin.lerp(desired,reach)
        solve_arm(side,target)
    a,b,t=sample(detail['samples'],phase)
    for s,side in enumerate(['L','R']):
        for i in range(3):
            angle=a['hair'][s*3+i]+(b['hair'][s*3+i]-a['hair'][s*3+i])*t
            p=rig.pose.bones[f'Braid.{side}.{i+1:02}'];rest=p.bone.matrix_local.to_3x3()
            # Forward/back swing plus a small outward arc prevents shoulder intersections.
            axis=Vector((1,(-.24 if side=='L' else .24),0)).normalized()
            limit=.40 if i==0 else .65
            gain=.45 if i==0 else .65
            q=Quaternion(axis,max(-limit,min(limit,-angle*gain))*weight)
            rotation(p,(rest.inverted()@q.to_matrix()@rest).to_quaternion())
    bpy.context.view_layer.update()
    evaluated=mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    floor=min((evaluated.matrix_world@v.co).z for v in evaluated.data.vertices)
    if floor<.003 and 0<phase<1:
        hips.location+=hips.bone.matrix_local.to_3x3().inverted()@Vector((0,0,.003-floor))
    floors.append(floor)
    for p in rig.pose.bones:
        p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=p.name)
        if not p.name.startswith('Braid'):p.keyframe_insert('location',frame=frame,group=p.name)
for c in curves():
    for key in c.keyframe_points:
        if 'Braid' in c.data_path or 156<=key.co.x<=180:key.interpolation='LINEAR'
assert unrelated()==before,'unrelated body keys changed'
assert np.array_equal(geometry,np.array([v.co[:] for v in mesh.data.vertices]))
selected_ids=set(selected['L']+selected['R'])
assert all(original_weights[v.index]==[(g.group,g.weight) for g in v.groups] for v in mesh.data.vertices if v.index not in selected_ids)
assert all(abs(sum(g.weight for g in v.groups)-1)<.0001 for v in mesh.data.vertices)
assert all(len(v.groups)<=4 for v in mesh.data.vertices)
checkpoints=json.loads((ASSETS/'attack/pose-checkpoints.json').read_text())
entry=next(x for x in checkpoints if x['slug']=='naia');entry['poses']=[]
for phase in [0,.2,.4,.6,.8,1]:
    frame=156+phase*24;scene.frame_set(int(frame),subframe=frame-int(frame));bpy.context.view_layer.update()
    entry['poses'].append({n:list(rig.matrix_world@rig.pose.bones[n].head) for n in ['Hips','Head','Hand.L','Hand.R']})
(ASSETS/'attack/pose-checkpoints.json').write_text(json.dumps(checkpoints))
scene['naia_braid_source']='Attack1_1 Hair_Tail_L/R_0..2; independent three-joint chains'
scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(blend));export_fbx(mesh,rig,ASSETS/'naia.fbx')
for path in [ASSETS/'naia-rig.json',ASSETS/'manifest.json']:
    data=json.loads(path.read_text(encoding='utf8'));item=next(x for x in data if x['slug']=='naia') if isinstance(data,list) else data
    item['bones']=len(rig.data.bones);item['braid_chains']={s:[f'Braid.{s}.{i+1:02}' for i in range(3)] for s in ['L','R']}
    path.write_text(json.dumps(data,indent=2,ensure_ascii=False),encoding='utf8')
(OUT/'rig-verification.json').write_text(json.dumps(dict(bones=len(rig.data.bones),weighted_vertices={s:len(v) for s,v in selected.items()},centers={s:[list(p) for p in v] for s,v in centers.items()},geometry_preserved=True,unrelated_weights_preserved=True,unrelated_body_keys_preserved=True,source_sha256=detail['sha256']),indent=2))
print('NAIA_BRAIDS_ATTACK_OK',len(rig.data.bones),{s:len(v) for s,v in selected.items()},flush=True)
