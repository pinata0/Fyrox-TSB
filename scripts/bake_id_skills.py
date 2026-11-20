"""Split Id's pod into editable skinned panels, rig hair and bake archived E/Q.

Reads current assets, preserves facial geometry/keys and unrelated clips. Re-running
uses the recorded baseline in artifacts/id-skills/before (never overwrites it).
"""
import bpy, bmesh, json, math, sys, shutil, hashlib
from pathlib import Path
from mathutils import Vector, Quaternion, Matrix

ROOT=Path(__file__).resolve().parents[1]; sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx, resolve_character_textures
ASSETS=ROOT/'assets/characters'; WORK=ROOT/'artifacts/id-skills'; BEFORE=WORK/'before'
BEFORE.mkdir(parents=True,exist_ok=True)
for name in ['id.blend','id.fbx','id.morphs.json','id-rig.json']:
    if not (BEFORE/name).exists(): shutil.copy2(ASSETS/name,BEFORE/name)
bpy.ops.wm.open_mainfile(filepath=str(BEFORE/'id.blend'))
scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects['id_SkinnedMesh']
resolve_character_textures([o for o in scene.objects if o.type=='MESH'])
for im in bpy.data.images:
    if im.source=='FILE':
        p=ASSETS/Path(bpy.path.abspath(im.filepath)).name
        if p.is_file(): im.filepath=str(p)
# Cache the original posture before inserting keys; newly inserted keys must not
# feed back into the interpolation used for later frames.
posture={}
for frame in range(154):
    scene.frame_set(frame)
    posture[frame]={n:rig.pose.bones[n].rotation_quaternion.copy() for n in ['Torso','Head']}
scene.frame_set(0)
for n,angle in [('Torso',-.28),('Head',.12)]:
    p=rig.pose.bones[n];p.rotation_quaternion=Quaternion((1,0,0),angle)@p.rotation_quaternion
bpy.context.view_layer.update()
base={p.name:p.matrix_basis.copy() for p in rig.pose.bones}
hip=rig.pose.bones['Hips'].matrix.copy()
mount=bpy.data.objects['Crescent'];original_faces=len(mount.data.polygons)

# Six actual mesh objects preserve the original material/UV loops. Each face is
# assigned once; the seat, two lamps and three shell sectors have their own bones.
def region(co):
    x,y,z=co
    if y<.10 and z<.46: return 'Lantern.L' if x<0 else 'Lantern.R'
    if abs(x)<.47 and .36<z<1.43 and .05<y<.91: return 'Seat'
    if z>1.40: return 'Crown'
    if y<.43: return 'Bow'
    return 'Back'
groups={n:[] for n in ['Seat','Bow','Back','Crown','Lantern.L','Lantern.R']}
for p in mount.data.polygons:
    center=sum((mount.data.vertices[i].co for i in p.vertices),Vector())/len(p.vertices)
    groups[region(center)].append(p.index)
assert all(groups.values()),{n:len(f) for n,f in groups.items()}
parts={}
for name,indices in groups.items():
    obj=mount.copy();obj.data=mount.data.copy();scene.collection.objects.link(obj);obj.name='IdPodMesh.'+name
    bm=bmesh.new();bm.from_mesh(obj.data);bm.faces.ensure_lookup_table();keep=set(indices)
    bmesh.ops.delete(bm,geom=[f for f in bm.faces if f.index not in keep],context='FACES')
    bmesh.ops.delete(bm,geom=[v for v in bm.verts if not v.link_faces],context='VERTS')
    bm.to_mesh(obj.data);bm.free()
    parts[name]=obj
bpy.data.objects.remove(mount,do_unlink=True)
assert sum(len(o.data.polygons) for o in parts.values())==original_faces

bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
for name,obj in parts.items():
    bone=rig.data.edit_bones.new('IdPod.'+name)
    bone.head=sum((v.co for v in obj.data.vertices),Vector())/len(obj.data.vertices)
    bone.tail=bone.head+Vector((0,0,.10));bone.parent=rig.data.edit_bones['CrescentRoot']
for side,x in [('L',-.30),('C',0),('R',.30)]:
    for seg in range(3):
        b=rig.data.edit_bones.new(f'IdHair.{side}.{seg+1}')
        b.head=(x,.24,1.50-seg*.29);b.tail=(x,.24,1.21-seg*.29)
        b.parent=rig.data.edit_bones['Hair' if seg==0 else f'IdHair.{side}.{seg}']
bpy.ops.object.mode_set(mode='OBJECT')
for name,obj in parts.items():
    obj.vertex_groups.clear();obj.vertex_groups.new(name='IdPod.'+name).add(list(range(len(obj.data.vertices))),1,'REPLACE')
    obj['motion_source']='Skill1_1 / Ultimate1_1';obj['part']=name

# Transfer only pre-existing hair weights, retaining body/facial weights verbatim.
hair_index=body.vertex_groups['Hair'].index
hair_groups={(s,k):body.vertex_groups.new(name=f'IdHair.{s}.{k+1}') for s in ['L','C','R'] for k in range(3)}
hair_count=0
for v in body.data.vertices:
    weight=next((g.weight for g in v.groups if g.group==hair_index),0)
    if weight<.001:continue
    influence=max(0,min(1,(1.55-v.co.z)/.25))
    if influence==0:continue
    side='L' if v.co.x<-.16 else 'R' if v.co.x>.16 else 'C'
    step=max(0,min(2,(1.43-v.co.z)/.29));lo=int(step);hi=min(2,lo+1);f=step-lo
    body.vertex_groups['Hair'].add([v.index],weight*(1-influence),'REPLACE')
    hair_groups[side,lo].add([v.index],weight*influence*(1-f),'REPLACE')
    if hi!=lo:hair_groups[side,hi].add([v.index],weight*influence*f,'REPLACE')
    hair_count+=1
assert hair_count>100
data=json.loads((ASSETS/'skill/id.json').read_text());extra=[p.name for p in rig.pose.bones if p.name.startswith(('IdPod.','IdHair.'))]
for n in extra:base[n]=Matrix.Identity(4)
def smooth(t):t=max(0,min(1,t));return t*t*(3-2*t)
def set_rotation(p,q):
    p.rotation_mode='QUATERNION'
    if p.rotation_quaternion.dot(q)<0:q=-q
    p.rotation_quaternion=q
def sway(frame,t,strength=1,hair=None):
    for n in extra:
        p=rig.pose.bones[n]
        if n.startswith('IdHair.'):
            _,side,seg=n.split('.');seg=int(seg);idx=['L','C','R'].index(side)
            angle=.045*math.sin(t*2.35-seg*.72+idx*.9)*strength
            if hair:angle+=max(-.25,min(.25,hair[(idx*3+seg-1)%len(hair)]))*.65
            set_rotation(p,Quaternion((1,0,0),angle)@Quaternion((0,0,1),angle*.35))
        p.keyframe_insert(data_path='rotation_quaternion',frame=frame,group=n)
        p.keyframe_insert(data_path='location',frame=frame,group=n)

# Attack's rider is exactly the reclining idle pose. Nata supplies all firing motion.
scene.frame_set(1)  # Re-evaluate frame zero before adding the reclining offset once.
for f in range(529):
    scene.frame_set(f)
    if 156<=f<=180:
        for p in rig.pose.bones:
            p.matrix_basis=base[p.name]
            p.keyframe_insert(data_path='rotation_quaternion',frame=f,group=p.name)
            p.keyframe_insert(data_path='location',frame=f,group=p.name)
    if f<=153:
        for n,angle in [('Torso',-.28),('Head',.12)]:
            p=rig.pose.bones[n];p.rotation_quaternion=Quaternion((1,0,0),angle)@posture[f][n]
            p.keyframe_insert(data_path='rotation_quaternion',frame=f,group=n)
    # Seamless secondary motion in existing looping clips.
    clip=next(((a,b) for n,(a,b) in json.loads(scene['clips']).items() if a<=f<=b),None)
    t=(f-clip[0])/(clip[1]-clip[0])*math.tau/2.35 if clip else f/30
    sway(f,t)

clips=[(data['clips'][0],531),(None,642),(data['clips'][1],666)]
for clip,start in clips:
    count=len(clip['samples'])-1 if clip else 21
    for index in range(count+1):
        frame=start+index;t=index/30;scene.frame_set(frame)
        for p in rig.pose.bones:p.matrix_basis=base[p.name]
        if clip:
            s=clip['samples'][index];fade=smooth(index/4)*smooth((count-index)/8)
            lift=max(-.13,min(.46,s['mount'][1]*.80))*fade
            tilt=max(-.16,min(.16,-s['mount'][2]*.60))*fade
            p=rig.pose.bones['Mount'];p.location=base['Mount'].translation+p.bone.matrix_local.to_3x3().inverted()@Vector((0,0,lift))
            set_rotation(p,base['Mount'].to_quaternion()@Quaternion((1,0,0),tilt))
            bpy.context.view_layer.update()
            mount_delta=p.matrix@p.bone.matrix_local.inverted()
            rig.pose.bones['Hips'].matrix=mount_delta@hip
            for name,val,limit in [('Torso',s['torso'],.10),('Head',s['head'],.15)]:
                p=rig.pose.bones[name];set_rotation(p,base[name].to_quaternion()@Quaternion((1,0,0),max(-limit,min(limit,-val)) *fade))
            for j,name in enumerate(['Bow','Back','Crown']):
                p=rig.pose.bones['IdPod.'+name];v=s['panels'][j]
                delta=Vector((0,-v[0]*.42,v[1]*.42))*fade
                p.location=p.bone.matrix_local.to_3x3().inverted()@delta
                set_rotation(p,Quaternion((1,0,0),max(-.10,min(.10,-v[2]))*fade))
            p=rig.pose.bones['NataRoot'];v=s['nata'];p.location=base[p.name].translation+p.bone.matrix_local.to_3x3().inverted()@Vector((v[0]*.18,0,v[1]*.30-lift*.4))*fade
        for p in rig.pose.bones:
            p.keyframe_insert(data_path='rotation_quaternion',frame=frame,group=p.name)
            p.keyframe_insert(data_path='location',frame=frame,group=p.name)
            p.keyframe_insert(data_path='scale',frame=frame,group=p.name)
        sway(frame,t,1.8 if clip else 1,clip['samples'][index]['hair'] if clip else None)

clips=json.loads(scene['clips']);clips.update(IdSkill=[531,639],IdPulse=[642,663],IdUltimate=[666,826]);scene['clips']=json.dumps(clips)
scene.frame_end=826;scene.frame_set(0);scene.render.fps=30
report={'source_sha256':data['sha256'],'release':data['release'],'parts':{n:len(o.data.polygons) for n,o in parts.items()},'hair_vertices':hair_count,'hair_bones':9,'clips':clips}
scene['id_skills']=json.dumps(report)
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'id.blend'),relative_remap=False)
export_fbx(body,rig,ASSETS/'id.fbx')
(WORK/'asset-report.json').write_text(json.dumps(report,indent=2))
print('ID_SKILLS_BAKED',json.dumps(report),flush=True)
