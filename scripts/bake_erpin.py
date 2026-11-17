"""Retarget archived Attack2_1 and isolate feather weights on independent wing chains.

Starts from this task's untouched backup; preserves mesh/UVs and existing body keys.
"""
import bpy, json, math, sys
import numpy as np
from pathlib import Path
from mathutils import Vector, Matrix, Quaternion
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
OUT=ROOT/'artifacts/erpin-motion';ASSETS=ROOT/'assets/characters'
bpy.ops.wm.open_mainfile(filepath=str(OUT/'before/erpin.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];mesh=bpy.data.objects['erpin_SkinnedMesh']
scene.frame_set(0)
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
before={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points] for c in curves()}
geometry=[v.co[:] for v in mesh.data.vertices]
base={p.name:(p.rotation_quaternion.copy() if p.rotation_mode=='QUATERNION' else p.rotation_euler.to_quaternion(),p.location.copy(),p.scale.copy()) for p in rig.pose.bones}
def rot(p,q):
    if p.rotation_mode=='QUATERNION':
        if p.rotation_quaternion.dot(q)<0:q=-q
        p.rotation_quaternion=q
    else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)
def smooth(x):x=max(0,min(1,x));return x*x*(3-2*x)
def aim(name,end,target):
    p=rig.pose.bones[name];q=(rig.pose.bones[end].head-p.head).rotation_difference(target-p.head)
    p.matrix=Matrix.Translation(p.head)@q.to_matrix().to_4x4()@Matrix.Translation(-p.head)@p.matrix
    bpy.context.view_layer.update()
def limb(names,target,pole):
    upper,fore,hand=names;a=rig.pose.bones[upper].head.copy();b=rig.pose.bones[fore].head.copy();c=rig.pose.bones[hand].head.copy()
    l1=(b-a).length;l2=(c-b).length;d=target-a;distance=max(abs(l1-l2)+.001,min(l1+l2-.001,d.length));d.normalize();target=a+d*distance
    pole=Vector(pole);pole-=d*pole.dot(d);pole.normalize()
    along=(l1*l1-l2*l2+distance*distance)/(2*distance);height=math.sqrt(max(0,l1*l1-along*along))
    aim(upper,fore,a+d*along+pole*height);aim(fore,hand,target)

# The current delivered Meshy model is one connected surface. Select feather
# colors/topology, never the blonde hair, crown, cape or refined sleeves.
data=np.load(OUT/'vertices.npz');points=data['points'];colors=data['colors'];seed=data['mask']
neighbors=[[] for _ in points]
for e in mesh.data.edges:
    a,b=e.vertices;neighbors[a].append(b);neighbors[b].append(a)
ids=set(int(i) for i in np.flatnonzero(seed))
for _ in range(2):
    for i in list(ids):
        for j in neighbors[i]:
            x,y,z=points[j];r,g,b=colors[j]
            if .17<abs(x)<.68 and .27<y<.45 and .84<z<1.25 and b>r*.77 and g>r*.87:ids.add(j)
bpy.context.view_layer.objects.active=rig;rig.select_set(True);bpy.ops.object.mode_set(mode='EDIT')
for side,sign in [('L',-1),('R',1)]:
    # The feathers meet the original Head-weighted back surface. The root follows
    # that attachment exactly; only the dedicated feather chains receive inertia.
    root=rig.data.edit_bones.new('Wing.'+side);root.head=(sign*.205,.346,1.035);root.tail=(sign*.43,.355,1.06);root.parent=rig.data.edit_bones['Head']
    tip=rig.data.edit_bones.new('WingTip.'+side);tip.head=root.tail;tip.tail=(sign*.64,.36,1.15);tip.parent=root;tip.use_connect=True
bpy.ops.object.mode_set(mode='OBJECT')
wing_counts={}
for side,sign in [('L',-1),('R',1)]:
    gs=[mesh.vertex_groups.new(name=n+'.'+side) for n in ['Wing','WingTip']]
    selected=[i for i in ids if points[i,0]*sign>0];wing_counts[side]=len(selected);assert len(selected)>300
    for i in selected:
        # Seam remains pinned; deformation and inertia grow toward the feather tips.
        x=abs(points[i,0]);influence=smooth((x-.22)/.09);tip=smooth((x-.40)/.20)*.65
        v=mesh.data.vertices[i];weights={g.group:g.weight*(1-influence) for g in v.groups}
        weights[gs[0].index]=influence*(1-tip);weights[gs[1].index]=influence*tip
        weights=sorted(((g,w) for g,w in weights.items() if w>1e-7),key=lambda x:x[1],reverse=True)[:4];total=sum(w for _,w in weights)
        for g in list(v.groups):mesh.vertex_groups[g.group].remove([i])
        for g,w in weights:mesh.vertex_groups[g].add([i],w/total,'REPLACE')
    for n in ['Wing.','WingTip.']:rig.pose.bones[n+side].rotation_mode='QUATERNION'

clip=json.loads((ASSETS/'attack/erpin.json').read_text())[1]
checks=[]
for index,sample in enumerate(clip['samples']):
    frame=531+index;age=index/30;phase=index/50;scene.frame_set(frame)
    for n,(q,loc,scale) in base.items():p=rig.pose.bones[n];rot(p,q);p.location=loc;p.scale=scale
    weight=smooth(age/.18)*smooth((50/30-age)/.22)
    feet={side:rig.pose.bones['Foot.'+side].head.copy() for side in ['L','R']}
    for name,angle in sample['rotations'].items():
        if name.startswith(('UpperArm','Forearm','Hand','Foot','Thigh','Shin')):continue
        limit=.13 if name=='Hips' else .22
        value=max(-limit,min(limit,-angle*.45))*weight
        p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3();rot(p,base[name][0]@(rest.inverted()@Matrix.Rotation(value,3,'X')@rest).to_quaternion())
    hips=rig.pose.bones['Hips'];hips.location=base['Hips'][1]+hips.bone.matrix_local.to_3x3().inverted()@Vector((0,.045*weight,-.20*weight))
    bpy.context.view_layer.update()
    for side in ['L','R']:limb([n+'.'+side for n in ['Thigh','Shin','Foot']],feet[side],(0,-1,0))
    head=rig.pose.bones['Head'].head.copy()
    bite=max(max(0,1-abs((age-t)/.11)) for t in [11/30,17/30,23/30,29/30])
    for j,side in enumerate(['L','R']):
        hand=rig.pose.bones['Hand.'+side];origin=hand.head.copy();path=sample['hands'][j]
        # Anatomical left is Hand.R in this source rig. Keep the staff in the right.
        target=(head+Vector((.14,-.25-.035*bite,-.11+.045*bite))) if side=='R' else Vector((-.32,-.13+.035*path[0],.72-.15*weight))
        limb([n+'.'+side for n in ['UpperArm','Forearm','Hand']],origin.lerp(target,weight),(-1 if side=='L' else 1,-.5,-.65))
    for p in rig.pose.bones:
        if p.name.startswith('Wing'):continue
        p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=frame,group=p.name)
        p.keyframe_insert('location',frame=frame,group=p.name)
    if index in [0,11,17,23,29,50]:checks.append({'frame':frame,'head':list(head),'left_hand':list(rig.pose.bones['Hand.R'].head)})

# Damped inertia driven by the evaluated attachment, baked separately per clip.
# Two warm-up periods make looping clips seamless. No independent translations.
clips=[(0,60,True),(63,93,True),(96,120,True),(123,153,False),(156,180,False),(183,219,False),(222,282,False),(285,405,True),(408,420,False),(423,453,False),(456,528,False),(531,581,False)]
for start,end,loop in clips:
    angle=Vector((0,0,0));velocity=Vector((0,0,0));previous=None;old_vel=Vector((0,0,0));samples={}
    for frame in list(range(start,end+1))*(3 if loop else 1):
        scene.frame_set(frame);bpy.context.view_layer.update();p=rig.pose.bones['Head'];q=p.matrix.to_quaternion();pos=p.head.copy()
        if previous:
            pq,pp=previous;angular=(pq.inverted()@q).to_exponential_map()*30
            speed=(pos-pp)*30;accel=(speed-old_vel)*30;old_vel=speed
            force=Vector((-angular.x*5.5-accel.y*.10,-angular.z*3.5, -accel.z*.10))
        else:force=Vector((0,0,0))
        previous=(q,pos)
        for _ in range(4):
            velocity+=(force-angle*55-velocity*12)/120;angle+=velocity/120
        angle=Vector([max(-.30,min(.30,v)) for v in angle]);samples[frame]=angle.copy()
    if loop:samples[end]=samples[start].copy()
    for frame,angle in samples.items():
        scene.frame_set(frame)
        for side,sign in [('L',-1),('R',1)]:
            for stem,gain in [('Wing',1),('WingTip',.65)]:
                p=rig.pose.bones[stem+'.'+side];rest=p.bone.matrix_local.to_3x3()
                world=Quaternion((1,0,0),angle.x*gain)@Quaternion((0,0,1),sign*(angle.y+angle.z)*gain)
                p.rotation_quaternion=(rest.inverted()@world.to_matrix()@rest).to_quaternion()
                p.keyframe_insert('rotation_quaternion',frame=frame,group=p.name)
for c in curves():
    for k in c.keyframe_points:
        if k.co.x>=531 or 'Wing' in c.data_path:k.interpolation='LINEAR'
after={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<531] for c in curves()}
assert all(after[k]==v for k,v in before.items()),'existing body keys changed'
assert geometry==[v.co[:] for v in mesh.data.vertices],'geometry changed'
scene.frame_end=581;scene.frame_set(0);scene.render.fps=30
scene['erpin_attack2']='Attack2_1 531:581, four source bites, anatomical left hand'
scene['erpin_wings']='Pinned feather seam; Head attachment; Wing and WingTip independent chains; inertial rotation on all clips'
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'erpin.blend'))
export_fbx(mesh,rig,ASSETS/'erpin.fbx')
(OUT/'bake-verification.json').write_text(json.dumps({'geometry_preserved':True,'existing_body_keys_preserved':True,'wing_vertices':wing_counts,'clip_ranges':clips,'eat_checkpoints':checks},indent=2))
print('ERPIN_BAKE_OK',wing_counts,flush=True)
