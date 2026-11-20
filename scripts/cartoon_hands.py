"""Replace only current hand meshes with four rounded, two-segment digits.

Run with Blender --background --python scripts/cartoon_hands.py -- [slugs].
The current assets are snapshotted once; output stays in the review stage.
"""
import hashlib
import json
import math
import shutil
import sys
from pathlib import Path

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Quaternion, Vector

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from export_fyrox_fbx import export_fbx, resolve_character_textures

OUT = ROOT / 'artifacts/cartoon-hands'
STAGE = OUT / 'characters'
BACK = OUT / 'before'
ASSETS = ROOT / 'assets/characters'
SLUGS = 'vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
DIGITS = ('Index', 'Middle', 'Pinky', 'Thumb')
REVISION = 'four-digits-two-segments-v1'


def unit(v):
    return v / np.linalg.norm(v)


def fingerprint(obj):
    """Geometry, UVs, active weights and facial morphs on untouched meshes."""
    h = hashlib.sha256()
    h.update(np.array([v.co[:] for v in obj.data.vertices], dtype=np.float32).tobytes())
    h.update(str([(tuple(p.vertices), p.material_index) for p in obj.data.polygons]).encode())
    for uv in obj.data.uv_layers:
        h.update(np.array([x.uv[:] for x in uv.data], dtype=np.float32).tobytes())
    for v in obj.data.vertices:
        h.update(str(sorted((obj.vertex_groups[g.group].name, g.weight) for g in v.groups if g.weight > 0)).encode())
    if obj.data.shape_keys:
        for k in obj.data.shape_keys.key_blocks:
            h.update(k.name.encode())
            h.update(np.array([v.co[:] for v in k.data], dtype=np.float32).tobytes())
    return h.hexdigest()


def neutral(rig):
    rig.animation_data_clear()
    rig.data.pose_position = 'POSE'
    for b in rig.pose.bones:
        b.matrix_basis = Matrix.Identity(4)
    bpy.context.view_layer.update()


def pose(rig, profiles, amount=1):
    for p in profiles:
        for c in p['controls']:
            rig.pose.bones[c['name']].matrix_basis = Matrix.Identity(4)
    bpy.context.view_layer.update()
    for p in profiles:
        for c in p['controls']:
            bone = rig.pose.bones[c['name']]
            axis = rig.pose.bones[p['hand']].matrix.to_3x3() @ Vector(c['axis'])
            before = bone.matrix.copy()
            result = (Quaternion(axis, c['angle'] * amount).to_matrix() @ before.to_3x3()).to_4x4()
            result.translation = before.translation
            bone.matrix = result
            bpy.context.view_layer.update()


def ellipsoid(center, radii, axes, parts):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=28, ring_count=16)
    obj = bpy.context.object
    transform = np.column_stack(axes) @ np.diag(radii)
    for v in obj.data.vertices:
        v.co = Vector(center + transform @ np.array(v.co))
    parts.append(obj)


def skin_color(obj):
    mat = obj.data.materials[0]
    tex = next((n.image for n in mat.node_tree.nodes if n.type == 'TEX_IMAGE' and n.image), None)
    if tex and obj.data.uv_layers.active:
        pixels = np.array(tex.pixels[:]).reshape(tex.size[1], tex.size[0], 4)
        uv = np.array([v.uv[:] for v in obj.data.uv_layers.active.data])
        xy = np.clip((uv * np.array(tex.size)).astype(int), [0, 0], np.array(tex.size) - 1)
        return np.clip(np.quantile(pixels[xy[:, 1], xy[:, 0], :3], .85, axis=0), .35, .98)
    return np.array(mat.diffuse_color[:3])


def make_hand(slug, p, rig):
    old = bpy.data.objects[f'{slug}_RefinedHand.{p["side"]}']
    rgb = skin_color(old)
    name = old.name
    bpy.data.objects.remove(old, do_unlink=True)
    w, f, a, n = (np.array(p[k]) for k in ('wrist', 'forward', 'across', 'dorsal'))
    scale = p['radius'] / .013
    parts, chains = [], []
    # Keep the wrist/cuff seam and palm socket; the palm is soft and broad.
    ellipsoid(w + f * .029 * scale, [.041 * scale, .038 * scale, .018 * scale], [f, a, n], parts)
    ellipsoid(w - f * .024 * scale, [.055 * scale, .024 * scale, .019 * scale], [f, a, n], parts)
    for digit, across, ratio in [('Index', .0255, .94), ('Middle', 0, 1), ('Pinky', -.0255, .87)]:
        base = w + f * .059 * scale + a * across * scale
        tip = base + f * .043 * scale * ratio
        radius = .011 * scale
        ellipsoid((base + tip) / 2, [np.linalg.norm(tip-base)/2 + radius, radius, radius*.94], [f, a, n], parts)
        chains.append((digit, base, tip, [unit(np.cross(f, -n))]*2, [.48, 1.05]))
    base = w + f*.031*scale + a*.030*scale - n*.003*scale
    tf = unit(f*.42 + a*.90 - n*.035)
    tip = base + tf*.035*scale
    ta = unit(np.cross(tf, n)); tn = unit(np.cross(tf, ta))
    ellipsoid((base+tip)/2, [.035*scale/2 + .012*scale, .012*scale, .0115*scale], [tf, ta, tn], parts)
    dirs = [tf, unit(f*.5-a*.12-n*.86), unit(f*.79-a*.18-n*.58)]
    axes = [unit(np.cross(x,y)) for x,y in zip(dirs,dirs[1:])]
    angles = [math.acos(float(np.clip(np.dot(x,y),-1,1))) for x,y in zip(dirs,dirs[1:])]
    chains.append(('Thumb',base,tip,axes,angles))
    bpy.ops.object.select_all(action='DESELECT')
    for obj in parts: obj.select_set(True)
    bpy.context.view_layer.objects.active=parts[0]
    bpy.ops.object.join(); obj=parts[0]; obj.name=name
    mod=obj.modifiers.new('Continuous rounded hand','REMESH');mod.mode='VOXEL';mod.voxel_size=.0015*scale
    bpy.ops.object.modifier_apply(modifier=mod.name)
    mod=obj.modifiers.new('Soft finger webs','SMOOTH');mod.factor=.65;mod.iterations=4
    bpy.ops.object.modifier_apply(modifier=mod.name)
    mod=obj.modifiers.new('Hand triangle budget','DECIMATE');mod.ratio=.48
    bpy.ops.object.modifier_apply(modifier=mod.name)
    bm=bmesh.new();bm.from_mesh(obj.data)
    # Voxel remeshing can leave eight-vertex specks at grazing sphere overlaps.
    # Remove only tiny artifacts; a detached actual digit must fail loudly.
    pending=set(bm.verts);components=[]
    while pending:
        stack=[pending.pop()];part=[]
        while stack:
            vertex=stack.pop();part.append(vertex)
            for edge in vertex.link_edges:
                other=edge.other_vert(vertex)
                if other in pending:pending.remove(other);stack.append(other)
        components.append(part)
    components.sort(key=len,reverse=True)
    for part in components[1:]:
        assert len(part)<=32 and len(part)<len(components[0])*.005,(slug,p['side'],'detached digit')
        bmesh.ops.delete(bm,geom=part,context='VERTS')
    bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(obj.data);bm.free()
    for poly in obj.data.polygons:poly.use_smooth=True
    mat=bpy.data.materials.new(f'{slug}_cartoon_skin_{p["side"]}');mat.use_nodes=True
    mat.diffuse_color=(*rgb,1)
    mat.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=(*rgb,1)
    mat.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.75
    obj.data.materials.clear();obj.data.materials.append(mat)
    groups={}
    for v in obj.data.vertices:
        point=np.array(v.co);t=np.dot(point-w,f);ranked=[]
        for digit,base,tip,axes,angles in chains:
            d=tip-base;progress=np.dot(point-base,d)/np.dot(d,d)
            distance=np.linalg.norm(point-(base+d*np.clip(progress,0,1)))
            ranked.append((distance,digit,progress))
        ranked.sort();row={}
        # Each vertex follows at most two joints plus the palm. Blend only at webs.
        blends=np.exp(-np.array([r[0]-ranked[0][0] for r in ranked[:2]])**2/(2*(.003*scale)**2));blends/=blends.sum()
        for blend,(_,digit,progress) in zip(blends,ranked[:2]):
            influence=float(np.clip((progress+.20)/.36,0,1))
            if digit!='Thumb':influence*=float(np.clip((t/scale-.047)/.020,0,1))
            distal=float(np.clip((progress-.40)/.24,0,1))
            for key,value in [(p['hand'],1-influence),(f'{digit}1.{p["side"]}',influence*(1-distal)),(f'{digit}2.{p["side"]}',influence*distal)]:
                row[key]=row.get(key,0)+float(blend)*value
        wrist=float(np.clip(-t/(scale*.04),0,1))
        row={k:v*(1-wrist) for k,v in row.items()};row[f'Forearm.{p["side"]}']=wrist
        row=sorted(((k,v) for k,v in row.items() if v>1e-5),key=lambda item:-item[1])[:4]
        total=sum(v for _,v in row)
        for key,value in row:
            if key not in groups:groups[key]=obj.vertex_groups.new(name=key)
            groups[key].add([v.index],value/total,'REPLACE')
    obj.parent=rig;mod=obj.modifiers.new('Two segment skin','ARMATURE');mod.object=rig
    inv=rig.data.bones[p['hand']].matrix_local.to_3x3().inverted()
    controls=[];definitions=[]
    for digit,base,tip,axes,angles in chains:
        joints=[base,base+(tip-base)*.53,tip]
        for i in range(2):
            name=f'{digit}{i+1}.{p["side"]}'
            definitions.append((name,joints[i],joints[i+1],n,p['hand'] if i==0 else f'{digit}1.{p["side"]}'))
            controls.append(dict(name=name,axis=list(inv@Vector(axes[i])),angle=angles[i]))
    p.update(controls=controls,length=.113*scale,digit_count=4,segments_per_digit=2,hand_mesh_revision=REVISION,
             dimensions=dict(finger_length=.043*scale,finger_diameter=.022*scale,previous_finger_length=.070*scale,previous_finger_diameter=.0162*scale))
    return obj,definitions


def bake_hands(slug, hands):
    scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=8
    image=bpy.data.images.new(f'{slug}-cartoon-hands',width=512,height=512,alpha=False)
    image.filepath_raw=str(STAGE/f'{slug}-cartoon-hands.png');image.file_format='PNG'
    bpy.ops.object.select_all(action='DESELECT')
    for obj in hands:obj.select_set(True)
    bpy.context.view_layer.objects.active=hands[0]
    bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(angle_limit=1.15,island_margin=.025);bpy.ops.object.mode_set(mode='OBJECT')
    for obj in hands:
        mat=obj.data.materials[0];nodes=mat.node_tree.nodes;links=mat.node_tree.links
        bsdf=nodes['Principled BSDF'];out=nodes['Material Output']
        ao=nodes.new('ShaderNodeAmbientOcclusion');ao.inputs['Color'].default_value=mat.diffuse_color;ao.inputs['Distance'].default_value=.012;ao.only_local=True
        emit=nodes.new('ShaderNodeEmission');links.new(ao.outputs['Color'],emit.inputs[0]);links.new(emit.outputs[0],out.inputs['Surface'])
        tex=nodes.new('ShaderNodeTexImage');tex.image=image;nodes.active=tex
    scene.render.bake.margin=8
    bpy.ops.object.bake(type='EMIT');image.save()
    for obj in hands:
        mat=obj.data.materials[0];nodes=mat.node_tree.nodes;links=mat.node_tree.links
        links.new(nodes['Principled BSDF'].outputs[0],nodes['Material Output'].inputs['Surface'])
        links.new(nodes['Image Texture'].outputs['Color'],nodes['Principled BSDF'].inputs['Base Color'])
        mat['meshy_base_texture']=image.name
        nodes.remove(nodes['Emission']);nodes.remove(nodes['Ambient Occlusion'])


def run(slug):
    for folder in (STAGE,BACK):folder.mkdir(parents=True,exist_ok=True)
    for suffix in ('.blend','.fbx','-hands.json','-rig.json','.morphs.json'):
        path=ASSETS/f'{slug}{suffix}'
        if not (BACK/path.name).exists():shutil.copy2(path,BACK/path.name)
    bpy.ops.wm.open_mainfile(filepath=str(BACK/f'{slug}.blend'))
    rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects[f'{slug}_SkinnedMesh'];scene=bpy.context.scene
    resolve_character_textures([o for o in scene.objects if o.type=='MESH'])
    untouched={o.name:fingerprint(o) for o in scene.objects if o.type=='MESH' and '_RefinedHand.' not in o.name}
    scene.frame_set(0);rig.data.pose_position='REST'
    profiles=json.loads((BACK/f'{slug}-hands.json').read_text())
    hands=[];definitions=[]
    for p in profiles:
        hand,defs=make_hand(slug,p,rig);hands.append(hand);definitions+=defs
    removed={b.name for b in rig.data.bones if b.name.startswith('Ring') or (b.name.startswith(DIGITS) and b.name.split('.')[0].endswith('3'))}
    for obj in scene.objects:
        if obj.type=='MESH':
            for name in removed:
                group=obj.vertex_groups.get(name)
                if group:
                    assert not any(any(g.group==group.index and g.weight>0 for g in v.groups) for v in obj.data.vertices),(obj.name,name,'unexpected live old weights')
                    obj.vertex_groups.remove(group)
    bpy.ops.object.select_all(action='DESELECT');rig.select_set(True);bpy.context.view_layer.objects.active=rig
    bpy.ops.object.mode_set(mode='EDIT')
    for name in removed:rig.data.edit_bones.remove(rig.data.edit_bones[name])
    for name,head,tail,normal,parent in definitions:
        bone=rig.data.edit_bones[name];bone.head=head;bone.tail=tail;bone.align_roll(Vector(normal));bone.parent=rig.data.edit_bones[parent];bone.use_connect=False
    bpy.ops.object.mode_set(mode='OBJECT')
    assert untouched=={name:fingerprint(bpy.data.objects[name]) for name in untouched},'non-hand surface changed'
    from review_cartoon_hands import fit
    fitting=[dict(side=p['side'],digits=fit(rig,p,obj)) for p,obj in zip(profiles,hands)]
    bake_hands(slug,hands)
    record=json.loads((BACK/f'{slug}-rig.json').read_text())
    record.update(hands=profiles,bones=len(rig.data.bones),cartoon_hands=dict(revision=REVISION,digits=4,segments=2,untouched_meshes=untouched,grip_fit=fitting))
    (STAGE/f'{slug}-hands.json').write_text(json.dumps(profiles,indent=2))
    (STAGE/f'{slug}-rig.json').write_text(json.dumps(record,indent=2))
    scene['hand_rig']=json.dumps(profiles);scene['hand_mesh_revision']=REVISION
    rig.data.pose_position='POSE';scene.frame_set(0)
    for k in body.data.shape_keys.key_blocks:k.value=0
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    export_fbx(body,rig,STAGE/f'{slug}.fbx')
    print('CARTOON_HANDS_STAGED',slug,len(rig.data.bones),flush=True)


if __name__=='__main__':
    for slug in (sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS):run(slug)
