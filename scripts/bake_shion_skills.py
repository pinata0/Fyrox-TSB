"""Append Shion's four native-duration clips without altering delivered animation/geometry."""
import bpy, json, math, sys, shutil
from pathlib import Path
from mathutils import Vector, Matrix
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';WORK=ROOT/'artifacts/shion-skills';BEFORE=WORK/'before';BEFORE.mkdir(parents=True,exist_ok=True)
for name in ['shion.blend','shion.fbx','shion-rig.json']:
    if not (BEFORE/name).exists():shutil.copy2(ASSETS/name,BEFORE/name)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'shion.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['shion_SkinnedMesh']
meshes=[o for o in scene.objects if o.type=='MESH'];resolve_character_textures(meshes)
for image in bpy.data.images:
    if image.source=='FILE':
        path=ASSETS/Path(bpy.path.abspath(image.filepath)).name
        if path.is_file():image.filepath=str(path)
def curves():return rig.animation_data.action.layers[0].strips[0].channelbag(rig.animation_data.action_slot).fcurves
old={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves()}
geometry={o.name:[v.co[:] for v in o.data.vertices] for o in meshes}
scene.frame_set(0);base={p.name:p.matrix_basis.copy() for p in rig.pose.bones}
wrists={s:rig.pose.bones['Hand.'+s].head.copy() for s in ['L','R']}
def turn(name,angle):
    p=rig.pose.bones[name];rest=p.bone.matrix_local.to_3x3()
    q=base[name].to_quaternion()@(rest.inverted()@Matrix.Rotation(angle,3,'X')@rest).to_quaternion()
    if p.rotation_mode=='QUATERNION':
        if p.rotation_quaternion.dot(q)<0:q=-q
        p.rotation_quaternion=q
    else:p.rotation_euler=q.to_euler('XYZ',p.rotation_euler)
def aim(name,end,target):
    p=rig.pose.bones[name];current=rig.pose.bones[end].head-p.head;q=current.rotation_difference(target-p.head)
    p.matrix=Matrix.Translation(p.head)@q.to_matrix().to_4x4()@Matrix.Translation(-p.head)@p.matrix;bpy.context.view_layer.update()
def arm(side,target):
    upper='UpperArm.'+side;fore='Forearm.'+side;hand='Hand.'+side
    a=rig.pose.bones[upper].head.copy();b=rig.pose.bones[fore].head.copy();c=rig.pose.bones[hand].head.copy();l1=(b-a).length;l2=(c-b).length;d=target-a
    distance=max(abs(l1-l2)+.001,min(l1+l2-.002,d.length));d.normalize();target=a+d*distance
    pole=Vector((-1 if side=='L' else 1,.1,-.8));pole-=d*pole.dot(d);pole.normalize();along=(l1*l1-l2*l2+distance*distance)/(2*distance)
    aim(upper,fore,a+d*along+pole*math.sqrt(max(0,l1*l1-along*along)));aim(fore,hand,target)
data=json.loads((ASSETS/'skill/shion.json').read_text());ranges={}
for clip,start in zip(data['clips'],[531,598,631,674]):
    ranges[clip['clip']]=[start,start+len(clip['samples'])-1]
    for i,s in enumerate(clip['samples']):
        scene.frame_set(start+i)
        for name,m in base.items():rig.pose.bones[name].matrix_basis=m.copy()
        # Keep the source's continuous crouch across Load -> Loop -> End.
        for name,angle in s['rotations'].items():
            if name.startswith(('UpperArm','Forearm','Hand')):continue
            strength=.65 if name in ['Torso','Head'] else .75
            turn(name,max(-1.15,min(1.15,-angle*strength)))
        root=Vector((0,-.12*s['root'][0],.56*s['root'][1]))
        hips=rig.pose.bones['Hips'];hips.location=base['Hips'].to_translation()+hips.bone.matrix_local.to_3x3().inverted()@root
        bpy.context.view_layer.update()
        for j,side in enumerate(['L','R']):
            p=s['hands'][j];target=wrists[side]+root+Vector((0,max(-.24,min(.20,p[0]*.18)),max(-.14,min(.25,p[1]*.32))))
            arm(side,target)
        for p in rig.pose.bones:
            p.keyframe_insert('rotation_quaternion' if p.rotation_mode=='QUATERNION' else 'rotation_euler',frame=start+i,group=p.name)
            p.keyframe_insert('location',frame=start+i,group=p.name)
for c in curves():
    for k in c.keyframe_points:
        if k.co.x>=531:k.interpolation='LINEAR'
after={(c.data_path,c.array_index):[tuple(k.co) for k in c.keyframe_points if k.co.x<=528] for c in curves()}
assert old==after,'old animation keys changed'
assert all(geometry[o.name]==[v.co[:] for v in o.data.vertices] for o in meshes),'original geometry changed'
scene.frame_end=780;scene.frame_set(0);scene.render.fps=30;bpy.context.preferences.filepaths.save_version=0
scene['shion_skills']=json.dumps(ranges)
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'shion.blend'),relative_remap=False)
export_fbx(body,rig,ASSETS/'shion.fbx')
metadata=json.loads((BEFORE/'shion-rig.json').read_text());metadata['clips'].update(ranges)
(ASSETS/'shion-rig.json').write_text(json.dumps(metadata,indent=2)+'\n')
(WORK/'bake-verification.json').write_text(json.dumps({'old_keys_preserved':True,'geometry_preserved':True,'clips':ranges},indent=2))
print('SHION_BAKE_OK',ranges,flush=True)
