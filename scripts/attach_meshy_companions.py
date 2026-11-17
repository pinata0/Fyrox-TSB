"""Attach separately skinned companions/mounts to staged character rigs."""
import bpy
import bmesh
import json
import math
import sys
import numpy as np
from pathlib import Path
from mathutils import Vector, Quaternion
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from import_meshy_characters import STAGE, OUT, setup_scene
from export_fyrox_fbx import export_fbx

def add_mesh(source,name,height,position,rig,parent,creature=False):
    existing=set(bpy.data.objects)
    if source.suffix=='.glb':bpy.ops.import_scene.gltf(filepath=str(source))
    else:bpy.ops.import_scene.fbx(filepath=str(source))
    imported=set(bpy.data.objects)-existing
    mesh=max((o for o in imported if o.type=='MESH'),key=lambda o:len(o.data.vertices))
    bpy.context.view_layer.update();world=mesh.matrix_world.copy();mesh.parent=None;mesh.matrix_world=world
    mesh.modifiers.clear();mesh.vertex_groups.clear()
    bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True);bpy.context.view_layer.objects.active=mesh
    bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
    bm=bmesh.new();bm.from_mesh(mesh.data);bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.000001);bm.to_mesh(mesh.data);bm.free()
    mod=mesh.modifiers.new('Companion budget','DECIMATE');mod.ratio=min(1,16000/max(1,len(mesh.data.polygons)));bpy.ops.object.modifier_apply(modifier=mod.name)
    p=np.array([v.co[:] for v in mesh.data.vertices]);lo=p.min(0);hi=p.max(0)
    factor=height/(hi[2]-lo[2]);p=(p-np.array([(lo[0]+hi[0])*.5,(lo[1]+hi[1])*.5,lo[2]]))*factor+position
    mesh.data.vertices.foreach_set('co',p.astype(np.float32).ravel());mesh.name=name
    for o in imported:
        if o!=mesh:bpy.data.objects.remove(o,do_unlink=True)
    for mat in mesh.data.materials:
        mat.name=name+'_material';bsdf=mat.node_tree.nodes.get('Principled BSDF');bsdf.inputs['Roughness'].default_value=.85
        for socket in ['Metallic','Roughness','Normal']:
            for link in list(bsdf.inputs[socket].links):mat.node_tree.links.remove(link)
        for node in list(mat.node_tree.nodes):
            if node.type=='TEX_IMAGE' and node.image:
                if not any(l.to_socket==bsdf.inputs['Base Color'] for l in node.outputs['Color'].links):mat.node_tree.nodes.remove(node);continue
                image=node.image;image.scale(1024,1024);image.filepath_raw=str(STAGE/f'{name}-basecolor.png');image.file_format='PNG';image.save();image.pack()
    bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
    root=rig.data.edit_bones.new(name+'Root');root.head=position;root.tail=root.head+Vector((0,0,.1));root.parent=rig.data.edit_bones[parent]
    controls={root.name:np.array(position)}
    if creature:
        for label,pos in [('Head',(0,0,height*.65)),('Arm.L',(-height*.22,0,height*.52)),('Arm.R',(height*.22,0,height*.52)),('Leg.L',(-height*.18,0,height*.15)),('Leg.R',(height*.18,0,height*.15))]:
            b=rig.data.edit_bones.new(name+'.'+label);b.head=Vector(position)+Vector(pos);b.tail=b.head+Vector((0,0,.08));b.parent=root;controls[b.name]=np.array(b.head)
    bpy.ops.object.mode_set(mode='OBJECT')
    groups={n:mesh.vertex_groups.new(name=n) for n in controls}
    for i,co in enumerate(p):
        q=(co-position)/height;n=name+'Root';w=0
        if creature:
            if q[2]>.64:n=name+'.Head';w=min(1,(q[2]-.64)/.12)
            elif abs(q[0])>.23:n=name+('.Arm.L' if q[0]<0 else '.Arm.R');w=min(.9,(abs(q[0])-.23)/.15)
            elif q[2]<.2:n=name+('.Leg.L' if q[0]<0 else '.Leg.R');w=.8
        groups[name+'Root'].add([i],1-w,'REPLACE')
        if w:groups[n].add([i],w,'REPLACE')
    mesh.parent=rig;mod=mesh.modifiers.new('Independent companion skin','ARMATURE');mod.object=rig
    return mesh,list(controls)

def run(slug):
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'));scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects[f'{slug}_SkinnedMesh']
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body];controls=[]
    if slug=='chloe':
        mesh,bones=add_mesh(ROOT/'assets/character-sources/meshy/chloe-sebastian/chloe-sebastian-source.glb','Sebastian',1.25,np.array([0,.40,.67]),rig,'Torso',True);extras.append(mesh);controls+=bones
    elif slug=='ui':
        # Raise the rider above the saddle while keeping the frog independently rooted.
        for mesh in [body,*extras]:
            for key in mesh.data.shape_keys.key_blocks:
                for v in key.data:v.co.z+=.24
            for vertex,base in zip(mesh.data.vertices,mesh.data.shape_keys.key_blocks['Basis'].data):vertex.co=base.co
            mesh.data.update()
        bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
        for b in rig.data.edit_bones:
            if b.name not in ['Root','Mount']:b.head.z+=.24;b.tail.z+=.24
        bpy.ops.object.mode_set(mode='OBJECT')
        mesh,bones=add_mesh(ROOT/'assets/character-sources/meshy/ui-frog/ui-frog-source.glb','Frog',.66,np.array([0,-.04,0]),rig,'Mount',True);extras.append(mesh);controls+=bones
    else:
        for kind,height,pos in [('mount',1.22,[0,.16,.05]),('robot',.45,[.60,.06,.92])]:
            mesh,bones=add_mesh(STAGE/f'id-{kind}.fbx','Crescent' if kind=='mount' else 'Nata',height,np.array(pos),rig,'Mount');extras.append(mesh);controls+=bones
    for f in range(529):
        scene.frame_set(f)
        clip,start,end=next(((n,s,e) for n,(s,e) in json.loads(scene['clips']).items() if s<=f<=e),('Idle',0,60))
        t=(f-start)/max(1,end-start);cycle=t*math.tau
        for name in controls:
            p=rig.pose.bones[name];p.rotation_mode='XYZ';p.rotation_euler=(0,0,0);p.location=(0,0,0)
            if name.endswith('Root'):
                if name.startswith('Nata'):p.location.z=.025*math.sin(cycle);p.rotation_euler.z=.06*math.sin(cycle)
            elif '.Arm.' in name:
                sign=-1 if name.endswith('.L') else 1
                p.rotation_euler.x=.10*math.sin(cycle+sign)
                if clip=='Attack':p.rotation_euler.x-=.8*math.sin(math.pi*t)**2
            elif '.Head' in name:p.rotation_euler.z=.035*math.sin(cycle)
            elif '.Leg.' in name and clip in ['Walk','Run','Dance']:p.rotation_euler.x=.12*math.sin(cycle)
            p.keyframe_insert(data_path='rotation_euler',frame=f,group=name);p.keyframe_insert(data_path='location',frame=f,group=name)
        if slug in ['id','ui'] and clip!='Death':
            for side in ['L','R']:
                for part,angle in [('Thigh',-.9 if slug=='id' else -.6),('Shin',.7 if slug=='id' else .4)]:
                    p=rig.pose.bones[f'{part}.{side}'];p.rotation_quaternion=Quaternion((1,0,0),angle)@p.rotation_quaternion;p.keyframe_insert(data_path='rotation_quaternion',frame=f,group=p.name)
    scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'));export_fbx(body,rig,STAGE/f'{slug}.fbx',extras)
    record=json.loads((STAGE/f'{slug}-rig.json').read_text());record['bones']=len(rig.data.bones);record['companions']=[m.name for m in extras if not m.name.endswith('_Eyelids')]
    if slug=='ui':
        for key in ['eye_y','mouth_y']:record['face'][key]+=.24
    (STAGE/f'{slug}-rig.json').write_text(json.dumps(record,indent=2))
    setup_scene(body,OUT/f'{slug}-companions.png')
    print('MESHY_COMPANIONS_OK',slug,flush=True)

if __name__=='__main__':
    for slug in ['chloe','ui','id']:run(slug)
