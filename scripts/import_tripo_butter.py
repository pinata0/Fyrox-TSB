"""Assemble the three Tripo parts, keep the supplied skin, and bake TSB motions.

Run with Blender --background --python scripts/import_tripo_butter.py.
Outputs a review candidate in artifacts/tripo-butter, without publishing it.
"""
import bpy
import bmesh
import json
import math
import sys
import shutil
from pathlib import Path
from mathutils import Vector, Matrix, Euler
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'assets/character-sources/tripo/butter'
OUT = ROOT / 'artifacts/tripo-butter'
sys.path.insert(0, str(ROOT / 'scripts'))
import build_characters as author
from export_fyrox_fbx import export_fbx

def components(mesh):
    neighbors = [[] for _ in mesh.vertices]
    for e in mesh.edges:
        a,b=e.vertices; neighbors[a].append(b); neighbors[b].append(a)
    unseen=set(range(len(neighbors))); groups=[]
    while unseen:
        seed=unseen.pop(); group=[seed]; stack=[seed]
        while stack:
            for i in neighbors[stack.pop()]:
                if i in unseen:
                    unseen.remove(i);group.append(i);stack.append(i)
        groups.append(group)
    return sorted(groups,key=len,reverse=True)

def bounds(obj, indices=None):
    points=[obj.data.vertices[i].co for i in indices] if indices else [v.co for v in obj.data.vertices]
    return [[min(v[i] for v in points),max(v[i] for v in points)] for i in range(3)]

def import_part(kind):
    before=set(bpy.data.objects)
    bpy.ops.import_scene.fbx(filepath=str(SOURCE/kind/f'butter-{kind}.fbx'))
    objects=list(set(bpy.data.objects)-before)
    mesh=next(o for o in objects if o.type=='MESH')
    world=mesh.matrix_world.copy();mesh.parent=None;mesh.matrix_world=world
    bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True);bpy.context.view_layer.objects.active=mesh
    bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
    mesh.name=f'Butter_{kind}'
    # Joined meshes must share the same active UV name; otherwise the clothing UVs become zero-filled.
    mesh.data.uv_layers.active.name='UVMap'
    for material in mesh.data.materials:
        material.name=f'Butter_{kind}'
        bsdf=material.node_tree.nodes.get('Principled BSDF')
        bsdf.inputs['Roughness'].default_value=.82
        bsdf.inputs['Metallic'].default_value=0
        for n in material.node_tree.nodes:
            if n.type=='TEX_IMAGE' and n.image:
                assert Path(bpy.path.abspath(n.image.filepath)).is_file(),n.image.filepath
                n.image.name=f'butter-{kind}-basecolor'
    return mesh,objects

def lighting():
    scene=bpy.context.scene
    scene.world=bpy.data.worlds.new('World');scene.world.color=(.65,.65,.65)
    scene.render.engine='CYCLES';scene.cycles.samples=16
    scene.view_settings.view_transform='AgX';scene.view_settings.look='None'
    scene.render.resolution_x=640;scene.render.resolution_y=640;scene.render.resolution_percentage=100
    scene.render.image_settings.file_format='PNG';scene.render.film_transparent=True
    for p,power,size in [((2,-4,5),450,4),((-3,-1,3),280,3),((0,3,4),450,3)]:
        bpy.ops.object.light_add(type='AREA',location=p)
        light=bpy.context.object;light.data.energy=power;light.data.shape='DISK';light.data.size=size
        light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=(0,-5,1))
    camera=bpy.context.object;camera.name='Preview camera';camera.data.type='ORTHO';camera.data.ortho_scale=2.45;scene.camera=camera
    return scene,camera

def render(scene,camera,name,frame=0,position=(0,-5,1),target=(0,0,1)):
    scene.frame_set(frame);camera.location=position
    camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler()
    scene.render.filepath=str(OUT/name);bpy.ops.render.render(write_still=True)

def main():
    OUT.mkdir(parents=True,exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    body,objects=import_part('rigged');rig=next(o for o in objects if o.type=='ARMATURE')
    bpy.ops.object.select_all(action='DESELECT');rig.select_set(True);bpy.context.view_layer.objects.active=rig
    bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
    rig.name='TSB_Rig'
    # Coordinate-side names match the existing TSB animation and weapon socket convention.
    names={'mixamorig:Hips':'Hips','mixamorig:Spine':'Torso','mixamorig:Head':'Head'}
    for source,side in [('Left','R'),('Right','L')]:
        for a,b in [('Arm','UpperArm'),('ForeArm','Forearm'),('Hand','Hand'),('UpLeg','Thigh'),('Leg','Shin'),('Foot','Foot')]:
            names[f'mixamorig:{source}{a}']=f'{b}.{side}'
    for old,new in names.items():
        rig.data.bones[old].name=new
        if old in body.vertex_groups:body.vertex_groups[old].name=new
    bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
    root=rig.data.edit_bones.new('Root');root.head=(0,0,0);root.tail=(0,0,.06)
    rig.data.edit_bones['Hips'].parent=root
    for name,parent,head in [('Hair','Head',(0,.03,.75)),('Cape','Torso',(0,.04,.5)),('Tail','Hips',(0,.08,.35)),('Mount','Root',(0,0,.15))]:
        b=rig.data.edit_bones.new(name);b.head=head;b.tail=Vector(head)+Vector((0,0,.06));b.parent=rig.data.edit_bones[parent]
    bpy.ops.object.mode_set(mode='OBJECT')
    clothes,_=import_part('clothes');accessories,_=import_part('accessories')
    raw_components={}
    for kind,obj in [('clothes',clothes),('accessories',accessories)]:
        raw_components[kind]=[{'vertices':len(c),'bounds':bounds(obj,c)} for c in components(obj.data)]
    (OUT/'components.json').write_text(json.dumps(raw_components,indent=2),encoding='utf-8')
    for component in components(clothes.data):
        bb=bounds(clothes,component);center=Vector([(a+b)/2 for a,b in bb])
        for i in component:
            v=clothes.data.vertices[i]
            if center.z>.45:
                v.co=Vector((v.co.x*.66,v.co.y*.70-.085,v.co.z*.565))
            else:
                v.co=v.co*.565+Vector((0,-.045,0))
                v.co.x=(v.co.x-center.x*.565)*1.27+center.x*.565
                v.co.y=(v.co.y+.05)*1.45-.08
    for component in components(accessories.data):
        bb=bounds(accessories,component);center=Vector([(a+b)/2 for a,b in bb])
        for i in component:
            v=accessories.data.vertices[i];v.co=v.co*.58+Vector((0,-.06,.34))
            if center.z>.8 and abs(center.x)>.2:
                v.co.x+=.055 if center.x>0 else -.055;v.co.y-=.15
            elif center.z>.8:
                v.co.z+=.035;v.co.y+=.05
            elif len(component)==114:
                v.co.y-=.115;v.co.z-=.048
            elif len(component)==170:
                v.co.y-=.215;v.co.z-=.048
            else:
                v.co.y-=.055
    # Transfer the existing body skin by nearest surface with barycentric interpolation.
    body.data.calc_loop_triangles()
    triangles=[tuple(t.vertices) for t in body.data.loop_triangles]
    tree=BVHTree.FromPolygons([v.co for v in body.data.vertices],triangles,all_triangles=True)
    body_weights=[{body.vertex_groups[g.group].name:g.weight for g in v.groups} for v in body.data.vertices]
    def nearest_weights(co):
        location,normal,index,distance=tree.find_nearest(co)
        ids=triangles[index];a,b,c=[body.data.vertices[i].co for i in ids]
        bary=barycentric_transform(location,a,b,c,Vector((1,0,0)),Vector((0,1,0)),Vector((0,0,1)))
        weights={}
        for i,w in zip(ids,bary):
            for name,value in body_weights[i].items():weights[name]=weights.get(name,0)+max(w,0)*value
        return weights
    for obj in [clothes,accessories]:
        groups={b.name:obj.vertex_groups.new(name=b.name) for b in rig.data.bones}
        for component in components(obj.data):
            bb=bounds(obj,component);center=Vector([(a+b)/2 for a,b in bb])
            for i in component:
                co=obj.data.vertices[i].co
                if obj==accessories:
                    weights={'Head':1} if center.z>.72 else {'mixamorig:Spine2':1} if center.z>.55 else {'Hips':1}
                elif center.z>.3: # dress: chest follows torso, hem stays together at the hips
                    t=max(0,min(1,(co.z-.34)/.17))
                    weights={'Hips':1-t,'mixamorig:Spine2':t}
                else:
                    weights=nearest_weights(co)
                weights={n:w for n,w in weights.items() if w>1e-5}
                weights=dict(sorted(weights.items(),key=lambda pair:pair[1],reverse=True)[:4])
                total=sum(weights.values());assert total>0
                for name,w in weights.items():groups[name].add([i],w/total,'REPLACE')
        mod=obj.modifiers.new('Tripo skin','ARMATURE');mod.object=rig
    # Keep the strongest native influences within the engine's four-weight vertex format.
    for v in body.data.vertices:
        weights=sorted([(g.group,g.weight) for g in v.groups],key=lambda pair:pair[1],reverse=True)
        kept=weights[:4];total=sum(w for i,w in kept);assert total>0
        for i,w in weights[4:]:body.vertex_groups[i].remove([v.index])
        for i,w in kept:body.vertex_groups[i].add([v.index],w/total,'REPLACE')
    # Remove the body surfaces covered by opaque clothes to prevent skin poke-through in poses.
    bm=bmesh.new();bm.from_mesh(body.data);bm.faces.ensure_lookup_table()
    hidden=[]
    torso_names={'Hips','Torso','mixamorig:Spine1','mixamorig:Spine2'}
    for face in bm.faces:
        co=face.calc_center_median()
        ids=[v.index for v in face.verts]
        torso=sum(sum(g.weight for g in body.data.vertices[i].groups if body.vertex_groups[g.group].name in torso_names) for i in ids)/len(ids)
        if max(v.co.z for v in face.verts)<.213 or (.32<co.z<.515 and abs(co.x)<.11 and co.y<.03 and torso>.65):
            hidden.append(face)
    bmesh.ops.delete(bm,geom=hidden,context='FACES');bm.to_mesh(body.data);bm.free()
    for obj in [body,clothes,accessories]:
        for v in obj.data.vertices:v.co*=1.95
    bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
    # Connected bone heads share their parent's tail. Snapshot first to avoid scaling those joints twice.
    joint_positions={b.name:(b.head.copy()*1.95,b.tail.copy()*1.95) for b in rig.data.edit_bones}
    for b in rig.data.edit_bones:b.head,b.tail=joint_positions[b.name]
    bpy.ops.object.mode_set(mode='OBJECT')
    bpy.ops.object.select_all(action='DESELECT')
    for obj in [body,clothes,accessories]:obj.select_set(True)
    bpy.context.view_layer.objects.active=body;bpy.ops.object.join();body.name='butter_SkinnedMesh';body.parent=rig
    scene,camera=lighting()
    scene.render.fps=30;scene.frame_start=0;scene.frame_end=528
    render(scene,camera,'assembled-front.png')
    render(scene,camera,'assembled-side.png',position=(5,0,1))
    render(scene,camera,'assembled-back.png',position=(0,5,1))
    if '--fit-only' in sys.argv:
        bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'butter-fit.blend'));return
    # Generate the existing eleven motions, then change their local axes to the native Tripo bind pose.
    author.RIG=rig;author.animate(11)
    frames=[]
    for f in range(529):
        scene.frame_set(f)
        frames.append({pb.name:(pb.rotation_euler.copy(),pb.location.copy()) for pb in rig.pose.bones})
    rig.animation_data_clear()
    rig.animation_data_create();rig.animation_data.action=bpy.data.actions.new('TSB_Motions_Tripo')
    old_axes=Matrix.Rotation(math.pi/2,3,'X')
    converters={b.name:b.matrix_local.to_3x3().inverted()@old_axes for b in rig.data.bones}
    for pb in rig.pose.bones:pb.rotation_mode='QUATERNION'
    for f,poses in enumerate(frames):
        for pb in rig.pose.bones:
            rotation,location=poses[pb.name];c=converters[pb.name]
            pb.rotation_quaternion=(c@rotation.to_matrix()@c.inverted()).to_quaternion()
            pb.location=pb.bone.matrix_local.to_3x3().inverted()@location
            pb.keyframe_insert(data_path='rotation_quaternion',frame=f,group=pb.name)
            pb.keyframe_insert(data_path='location',frame=f,group=pb.name)
    scene['clips']=json.dumps(author.CLIPS);scene['selection_gesture']=author.SELECT_GESTURES[11]
    scene['source']='User-provided Tripo body rig, clothes and accessories'
    guide=bpy.data.images.load(str(ROOT/'assets/turnarounds/butter.png'));guide.pack()
    reference=bpy.data.objects.new('Turnaround_REFERENCE_only',None);scene.collection.objects.link(reference)
    reference.empty_display_type='IMAGE';reference.data=guide;reference.hide_render=True;reference.hide_viewport=True
    textures=OUT/'textures';textures.mkdir(exist_ok=True)
    for kind in ['rigged','clothes','accessories']:
        filename=f'butter-{kind}-basecolor.jpg'
        shutil.copy2(SOURCE/kind/f'butter-{kind}.fbm'/filename,textures/filename)
        img=bpy.data.images[f'butter-{kind}-basecolor'];img.filepath=str(textures/filename);img.pack()
    scene.frame_set(0)
    camera.location=(2.8,-5,2.2)
    camera.rotation_euler=(Vector((0,0,1))-camera.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'butter.blend'))
    export_fbx(body,rig,OUT/'butter.fbx')
    render(scene,camera,'butter-preview.png',position=(2.8,-5,2.2))
    for name,frame in [('idle',0),('walk',70),('run',103),('jump',138),('attack',168),('ultimate',252),('dance',305),('select',480),('death',453)]:
        render(scene,camera,f'motion-{name}.png',frame)
    camera.data.ortho_scale=1.25;scene.render.resolution_x=256;scene.render.resolution_y=256
    render(scene,camera,'butter-portrait.png',0,position=(0,-5,1.52),target=(0,0,1.52))
    record={'id':11,'slug':'butter','vertices':len(body.data.vertices),'triangles':sum(len(p.vertices)-2 for p in body.data.polygons),
            'bones':len(rig.data.bones),'clips':author.CLIPS,'fps':30,'selection_gesture':author.SELECT_GESTURES[11],
            'source':'Tripo (user-provided)','source_manifest':'assets/character-sources/tripo/butter/source-files.json',
            'textures':[f'assets/characters/butter-{k}-basecolor.jpg' for k in ['rigged','clothes','accessories']]}
    (OUT/'candidate.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    print('TRIPO_CANDIDATE_OK',flush=True)

if __name__=='__main__':main()
