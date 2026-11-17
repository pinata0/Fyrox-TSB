"""Assemble Naia, Erpin and Renua and retarget the eleven TSB motions (Blender)."""
import bpy
import bmesh
import json
import math
import shutil
import sys
import heapq
import numpy as np
from pathlib import Path
from mathutils import Vector, Matrix
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/tripo-roster-import'
sys.path.insert(0,str(ROOT/'scripts'))
import build_characters as author
from import_tripo_butter import components, bounds, lighting
from export_fyrox_fbx import export_fbx
SPECS=json.loads((ROOT/'scripts/tripo_roster_sources.json').read_text())

def apply_transform(obj):
    world=obj.matrix_world.copy();obj.parent=None;obj.matrix_world=world
    bpy.ops.object.select_all(action='DESELECT')
    obj.select_set(True);bpy.context.view_layer.objects.active=obj
    bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)

def import_part(slug,kind):
    before=set(bpy.data.objects)
    folder=ROOT/'assets/character-sources/tripo'/slug/kind
    bpy.ops.import_scene.fbx(filepath=str(folder/f'{slug}-{kind}.fbx'))
    imported=list(set(bpy.data.objects)-before)
    mesh=next(o for o in imported if o.type=='MESH')
    apply_transform(mesh)
    mesh.name=f'{slug}_{kind}'
    mesh.data.uv_layers.active.name='UVMap'
    for material in mesh.data.materials:
        material.name=f'{slug}_{kind}'
        bsdf=material.node_tree.nodes.get('Principled BSDF')
        bsdf.inputs['Roughness'].default_value=.82
        bsdf.inputs['Metallic'].default_value=0
        for node in material.node_tree.nodes:
            if node.type=='TEX_IMAGE' and node.image:
                path=folder/f'{slug}-{kind}.fbm'/f'{slug}-{kind}-basecolor.jpg'
                node.image.filepath=str(path);node.image.reload()
                node.image.name=f'{slug}-{kind}-basecolor'
    if kind!='rigged':
        mesh.modifiers.clear();mesh.vertex_groups.clear()
        for obj in imported:
            if obj!=mesh:bpy.data.objects.remove(obj,do_unlink=True)
    if kind=='clothes':
        if slug=='naia':
            # The clothes export includes grey mannequin legs and their backing
            # panel. The illustration has bare legs; use the native skin instead.
            scaffold=set()
            for ids in components(mesh.data):
                bb=bounds(mesh,ids)
                if bb[2][0]<.01 and bb[2][1]<.60:scaffold.update(ids)
            assert scaffold,'missing Naia mannequin geometry'
            bm=bmesh.new();bm.from_mesh(mesh.data);bm.verts.ensure_lookup_table()
            bmesh.ops.delete(bm,geom=[bm.verts[i] for i in scaffold],context='VERTS')
            bm.to_mesh(mesh.data);bm.free();mesh.data.update()
        # Add arm cross-sections before fitting. The low-poly source sleeves have
        # long triangles that otherwise cut across the curved shoulder surface.
        bm=bmesh.new();bm.from_mesh(mesh.data)
        edges=[e for e in bm.edges if all(abs(v.co.x)>.12 and v.co.z>.45 for v in e.verts)]
        bmesh.ops.subdivide_edges(bm,edges=edges,cuts=1,use_grid_fill=True)
        if slug!='naia':
            edges=[e for e in bm.edges if all(abs(v.co.x)<.17 and v.co.z<.52 for v in e.verts)]
            bmesh.ops.subdivide_edges(bm,edges=edges,cuts=1,use_grid_fill=True)
        bm.to_mesh(mesh.data);bm.free();mesh.data.update()
        if slug=='renua':
            # Reuse the native fitted shorts below the waist. The separate AI
            # briefs have a much higher leg opening and leave a gap to the legs.
            shorts=next(ids for ids in components(mesh.data)
                        if .53<bounds(mesh,ids)[2][0]<.57 and .72<bounds(mesh,ids)[2][1]<.78)
            member=set(shorts)
            faces=[p for p in mesh.data.polygons if all(i in member for i in p.vertices)]
            swatch=min(faces,key=lambda p:(p.center-Vector((0,-.16,.68))).length)
            mesh['shorts_swatch_uv']=list(sum((mesh.data.uv_layers.active.data[i].uv for i in swatch.loop_indices),Vector((0,0)))/len(swatch.loop_indices))
            bm=bmesh.new();bm.from_mesh(mesh.data);bm.verts.ensure_lookup_table()
            remove=[f for f in bm.faces if all(v.index in member and v.co.z<(.415-.015)/.62 for v in f.verts)]
            bmesh.ops.delete(bm,geom=remove,context='FACES');bm.to_mesh(mesh.data);bm.free();mesh.data.update()
    if slug=='renua' and kind=='accessories':
        # The coat already contains both complete tassels. Retain only the head accessories here.
        duplicate=set()
        for ids in components(mesh.data):
            if bounds(mesh,ids)[2][1]<.28:duplicate.update(ids)
        bm=bmesh.new();bm.from_mesh(mesh.data);bm.verts.ensure_lookup_table()
        bmesh.ops.delete(bm,geom=[bm.verts[i] for i in duplicate],context='VERTS')
        bm.to_mesh(mesh.data);bm.free()
    return mesh,imported

def transform_component(obj,ids,scale,offset=(0,0,0),rotation=None):
    scale=Vector((scale,)*3 if isinstance(scale,(float,int)) else scale)
    for i in ids:
        point=obj.data.vertices[i].co
        point=Vector([point[a]*scale[a] for a in range(3)])
        if rotation is not None:point=rotation@point
        obj.data.vertices[i].co=point+Vector(offset)

def fit_erpin_crown(obj,ids,unit_scale=1.0):
    """Match the small crown in the fullbody/individual illustration.

    Transform the complete crown together, including its separate gem and
    filigree islands. Anchor the lower band, then seat it on the narrower scalp.
    """
    anchor=Vector((0,.03,.96))*unit_scale
    rise=Vector((0,0,.03))*unit_scale
    for i in ids:
        relative=obj.data.vertices[i].co-anchor
        obj.data.vertices[i].co=anchor+Vector((relative.x*.62,relative.y*.62,relative.z*.52))+rise

def fit_renua_head_accessories(obj,ids,unit_scale=1.0):
    """Fit earphones over the hair and float the complete halo behind it.

    Coordinates below are delivered meters (the importer applies 1.85 later).
    Move connected ornaments together so gems and hanging links stay joined.
    """
    factor=unit_scale/1.85
    selected=set(ids)
    regions={'halo':[], 'earphones':[], 'fins':[]}
    for island in components(obj.data):
        members=[i for i in island if i in selected]
        if not members:continue
        bb=bounds(obj,members)
        center=Vector([(a+b)*.5/factor for a,b in bb])
        region='halo' if center.y>.2 else 'fins' if bb[2][1]/factor>1.34 else 'earphones'
        regions[region].extend(members)
        sign=1 if center.x>0 else -1
        for i in members:
            p=obj.data.vertices[i].co/factor
            if region=='halo':
                p=Vector((p.x*1.60, p.y+.30, (p.z-1.865)*1.45+2.25))
            elif region=='fins':
                p=Vector((sign*.460+(p.x-sign*.378)*1.25,
                          (p.y+.09)*1.5-.35, (p.z-1.305)*2.60+1.56))
            else:
                p=Vector((sign*.490+(p.x-sign*.464)*1.60,
                          p.y*1.5-.35, (p.z-1.285)*2.50+1.485))
            obj.data.vertices[i].co=p*factor
    return regions


def fit(slug,kind,obj):
    groups=components(obj.data)
    assignments={}
    for ids in groups:
        bb=bounds(obj,ids);center=Vector([(a+b)/2 for a,b in bb])
        rigid=None
        if slug=='naia':
            if kind=='clothes':
                if center.z>.7:
                    transform_component(obj,ids,(.64,.66,.62),(0,-.025,-.015))
                    for i in ids:
                        co=obj.data.vertices[i].co
                        blend=max(0,min(1,(abs(co.x)-.09)/.055))
                        co.z+=.04*blend;co.y+=.025*blend
                elif center.z>.35:
                    transform_component(obj,ids,(.57,.53,.48),(0,-.02,.12))
                else:
                    transform_component(obj,ids,(.66,.55,.64),(0,-.005,.015))
                if center.y>.10 and center.z<.7:
                    rigid='Hips'
            elif kind=='head-hat':
                transform_component(obj,ids,(.68,.60,.40),(0,.025,.858))
                rigid='Head'
            elif kind=='bracelet':
                # The loop's source plane is XZ. Rotate it around the wrist's X axis.
                transform_component(obj,ids,.085,(.286,-.014,.557),Matrix.Rotation(math.pi/2,3,'Z'))
                rigid='Forearm.R'
            elif kind=='sandals':
                sign=1 if center.x>0 else -1
                transform_component(obj,ids,(.19,.15,.18),(sign*.013,-.03,-.005))
                rigid='Foot.R' if sign>0 else 'Foot.L'
        elif slug=='erpin':
            if kind=='hair':
                transform_component(obj,ids,(.60,.60,.72),(0,.025,.30))
                for i in ids:
                    co=obj.data.vertices[i].co
                    # The reference curls reach the brows; the generated fringe is too short.
                    front=max(0,min(1,(-co.y-.055)/.065))
                    cap=max(0,min(1,(co.z-.74)/.07))
                    co.z+=(co.z-1.03)*.45*front*cap
                    if co.z<.68:
                        co.z-=.18*min(1,(.68-co.z)/.38)
            elif kind=='clothes':
                if bb[2][1]<.16:
                    transform_component(obj,ids,(.65,.83,.65),(0,-.023,0))
                    rigid='Foot.R' if center.x>0 else 'Foot.L'
                elif center.z<.45:
                    transform_component(obj,ids,(.66,.9,.64),(0,-.025,.04))
                else:
                    transform_component(obj,ids,(.61,.67,.57),(0,-.02,.035))
                    for i in ids:
                        co=obj.data.vertices[i].co
                        co.z+=.065*max(0,min(1,(abs(co.x)-.09)/.06))
            elif kind=='cape':
                transform_component(obj,ids,(.53,.5,.49),(0,.025,.12))
                rigid='Cape' if center.z<.7 else 'mixamorig:Spine2'
            elif kind=='wings':
                transform_component(obj,ids,.70,(0,.18,.43))
                rigid='Wing.R' if center.x>0 else 'Wing.L'
            elif kind=='accessories':
                if center.z>.64:
                    transform_component(obj,ids,.66,(0,.015,.53));rigid='Head'
                elif center.z>.3:
                    transform_component(obj,ids,.19,(.29,-.03,.47));rigid='Hand.R'
                else:
                    transform_component(obj,ids,.38,(0,-.075,.43));rigid='mixamorig:Spine2'
        else:
            if kind=='hair':
                transform_component(obj,ids,(.48,.44,.60),(0,.025,.645))
            elif kind=='clothes':
                if center.z<.3 and abs(center.x)>.2:
                    transform_component(obj,ids,(.64,.62,.64),(0,.065,0));rigid='Cape'
                elif bb[2][1]<.52 and abs(center.x)<.15:
                    transform_component(obj,ids,(.80,.65,.70),(0,.06,0))
                else:
                    transform_component(obj,ids,(.66,.64,.62),(0,.065,.015))
                    if len(ids)>4000:
                        sleeve_ids=rotate_renua_sleeves(obj,ids)
                        obj['sleeve_vertices']=list(sleeve_ids)
                        for i in ids:
                            if i not in sleeve_ids and obj.data.vertices[i].co.z>.45:
                                assignments[i]={'mixamorig:Spine2':1.0}
            elif kind=='accessories':
                if center.y>.04:
                    transform_component(obj,ids,.80,(0,.12,.395));rigid='Head'
                elif center.z>.5:
                    sign=1 if center.x>0 else -1
                    transform_component(obj,ids,.46,(sign*.135,-.01,.48));rigid='Head'
                elif center.z>.28:
                    # Teal earrings fit just below the elf ears.
                    sign=1 if center.x>0 else -1
                    transform_component(obj,ids,.38,(sign*.16,0,.53));rigid='Head'
                else:
                    transform_component(obj,ids,.53,(0,.075,.04));rigid='Cape'
        if rigid:
            for i in ids:assignments[i]={rigid:1.0}
    if slug=='erpin' and kind=='accessories':
        fit_erpin_crown(obj,[i for i,weights in assignments.items() if weights=={'Head':1.0}])
    if slug=='renua' and kind=='accessories':
        fit_renua_head_accessories(obj,[i for i,weights in assignments.items() if weights=={'Head':1.0}])
    return assignments

def rotate_renua_sleeves(obj,coat_ids):
    """Find each sleeve by connectivity, then feather its rotation into the armhole."""
    positions=[v.co.copy() for v in obj.data.vertices]
    graph={i:[] for i in coat_ids}
    for edge in obj.data.edges:
        a,b=edge.vertices
        if a in graph and b in graph:graph[a].append(b);graph[b].append(a)
    sleeves=set()
    for sign in [1,-1]:
        remaining={i for i in coat_ids if sign*positions[i].x>.14};islands=[]
        while remaining:
            todo=[remaining.pop()];island=[]
            while todo:
                i=todo.pop();island.append(i)
                for j in graph[i]:
                    if j in remaining:remaining.remove(j);todo.append(j)
            if min(positions[i].z for i in island)>.30 and max(positions[i].z for i in island)>.45:
                islands.append(island)
        seed=max(islands,key=len)
        distances={i:0.0 for i in seed};queue=[(0.0,i) for i in seed];heapq.heapify(queue)
        while queue:
            distance,i=heapq.heappop(queue)
            if distance>distances[i]:continue
            for j in graph[i]:
                if sign*positions[j].x<.075 or positions[j].z<.40:continue
                candidate=distance+(positions[i]-positions[j]).length
                if candidate<.06 and candidate<distances.get(j,float('inf')):
                    distances[j]=candidate;heapq.heappush(queue,(candidate,j))
        pivot=Vector((sign*.105,.04,.53));rotation=Matrix.Rotation(-sign*math.pi/4,3,'Y')
        for i,distance in distances.items():
            t=distance/.06;weight=1-3*t*t+2*t*t*t
            co=positions[i];obj.data.vertices[i].co=co.lerp(pivot+rotation@(co-pivot),weight)
            sleeves.add(i)
    return sleeves

def fit_sleeves(clothes,rig,slug,assigned):
    """Center both sleeve tubes on the native arm, preserving their cuff openings."""
    original=[v.co.copy() for v in clothes.data.vertices]
    sleeve_mask=set(clothes.get('sleeve_vertices',range(len(original))))
    for sign,side in [(1,'R'),(-1,'L')]:
        sleeve=[i for i,co in enumerate(original) if i in sleeve_mask and sign*co.x>.095 and co.z>.45
                and (slug!='naia' or abs(co.x)>.17 or co.y>-.075)]
        points=np.array([original[i][:] for i in sleeve])
        xs=points[:,0]*sign;end=float(xs.max())
        samples=np.linspace(.12,end-.004,12);centers=[]
        for x in samples:
            section=[]
            for edge in clothes.data.edges:
                a,b=[original[v] for v in edge.vertices]
                if not any(i in sleeve_mask for i in edge.vertices):continue
                if min(a.z,b.z)<.44 or abs(a.x-b.x)<1e-8:continue
                t=(sign*x-a.x)/(b.x-a.x)
                if 0<=t<=1:
                    point=a.lerp(b,t)
                    if slug!='naia' or x>.17 or point.y>-.075:section.append(point[:])
            section=np.array(section)
            assert len(section),(slug,side,x)
            centers.append((section[:,1:].min(axis=0)+section[:,1:].max(axis=0))*.5)
        centers=np.array(centers)
        upper=rig.data.bones[f'UpperArm.{side}'];forearm=rig.data.bones[f'Forearm.{side}']
        wrist=forearm.tail_local
        for i in sleeve:
            co=original[i];x=sign*co.x
            blend=max(0,min(1,(x-.095)/.05))
            new_x=.105+(x-.105)*(abs(wrist.x)+.002-.105)/(end-.105)
            bone=upper if new_x<abs(forearm.head_local.x) else forearm
            t=(new_x-abs(bone.head_local.x))/(abs(bone.tail_local.x)-abs(bone.head_local.x))
            axis=bone.head_local.lerp(bone.tail_local,t)
            old_center=Vector((co.x,float(np.interp(x,samples,centers[:,0])),float(np.interp(x,samples,centers[:,1]))))
            radius_scale={'naia':1.50,'erpin':1.30,'renua':1.0}[slug]
            target=axis+(co-old_center)*radius_scale
            target.x=sign*new_x
            clothes.data.vertices[i].co=co.lerp(target,blend)
            assigned.pop(i,None)
            if new_x>abs(wrist.x)-.035:
                assigned[i]={f'Forearm.{side}':1.0}
        rim=[clothes.data.vertices[i].co for i in sleeve
             if sign*clothes.data.vertices[i].co.x>abs(wrist.x)-.012]
        rim_center=Vector([(min(p[a] for p in rim)+max(p[a] for p in rim))*.5 for a in range(3)])
        correction=wrist-rim_center;correction.x=0
        for i in sleeve:
            co=clothes.data.vertices[i].co
            blend=max(0,min(1,(sign*co.x-(abs(wrist.x)-.085))/.05))
            co+=correction*blend
        print('SLEEVE_AXIS_FITTED',slug,side,len(sleeve),end,abs(wrist.x)+.002,flush=True)

def leg_section(points,edges,height,sign):
    """Measure a leg tube with edge/plane intersections, not sparse vertex rows."""
    a=points[edges[:,0]];b=points[edges[:,1]]
    valid=(np.minimum(a[:,2],b[:,2])<=height)&(np.maximum(a[:,2],b[:,2])>=height)&(np.abs(b[:,2]-a[:,2])>1e-8)
    a=a[valid];b=b[valid]
    section=a+(b-a)*((height-a[:,2])/(b[:,2]-a[:,2]))[:,None]
    section=section[(np.abs(section[:,0])<.15)&((section[:,0]*sign>.002) if sign else True)]
    assert len(section)>3,('missing leg section',height,sign)
    lo=section[:,:2].min(axis=0);hi=section[:,:2].max(axis=0)
    return (lo+hi)*.5,(hi-lo)*.5


def fit_legs(body,rig,obj,slug,kind,assigned):
    """Fit independent stocking/boot shells to the original legs and skin weights."""
    if kind=='sandals':
        tags=obj.data.attributes.new('leg_part','INT','POINT')
        for v in obj.data.vertices:tags.data[v.index].value=5 if v.co.x>0 else 6
        return
    if slug=='naia':return
    native=np.array([v.co[:] for v in body.data.vertices])
    body_edges=np.array([e.vertices[:] for e in body.data.edges])
    original=np.array([v.co[:] for v in obj.data.vertices])
    tags=obj.data.attributes.new('leg_part','INT','POINT')
    body.data.calc_loop_triangles()
    triangles=[tuple(t.vertices) for t in body.data.loop_triangles]
    tree=BVHTree.FromPolygons([v.co for v in body.data.vertices],triangles,all_triangles=True)
    native_weights=[{body.vertex_groups[g.group].name:g.weight for g in v.groups} for v in body.data.vertices]
    for ids in components(obj.data):
        bb=bounds(obj,ids)
        if slug=='renua' and .39<bb[2][0]<.43 and .46<bb[2][1]<.49:
            # The generated briefs were behind and above the native pelvis.
            # Fit the upper waist panel over the native fitted shorts.
            member=set(ids)
            edges=np.array([e.vertices[:] for e in obj.data.edges if all(i in member for i in e.vertices)])
            heights=np.linspace(bb[2][0]+.004,bb[2][1]-.004,28)
            centers=[];radii=[];targets=[];target_radii=[]
            def shorts_height(z):return z
            for z in heights:
                center,radius=leg_section(original,edges,z,0)
                target,skin_radius=leg_section(native,body_edges,shorts_height(z),0)
                centers.append(center);radii.append(radius);targets.append(target);target_radii.append(skin_radius)
            for i in ids:
                co=original[i];z=co[2]
                center=np.array([np.interp(z,heights,np.array(centers)[:,a]) for a in range(2)])
                radius=np.array([np.interp(z,heights,np.array(radii)[:,a]) for a in range(2)])
                target=np.array([np.interp(z,heights,np.array(targets)[:,a]) for a in range(2)])
                skin_radius=np.array([np.interp(z,heights,np.array(target_radii)[:,a]) for a in range(2)])
                xy=target+(co[:2]-center)*np.maximum(1,(skin_radius+.007)/np.maximum(radius,.006))
                obj.data.vertices[i].co=(xy[0],xy[1],shorts_height(z))
            continue
        stocking=slug=='erpin' and .14<bb[2][0]<.20 and bb[2][1]<.34
        boot=bb[2][0]<.01 and bb[2][1]<(.11 if slug=='erpin' else .37) and max(abs(bb[0][0]),abs(bb[0][1]))<.13
        if not(stocking or boot):continue
        member=set(ids)
        edges=np.array([e.vertices[:] for e in obj.data.edges if all(i in member for i in e.vertices)])
        for sign,side in [(1,'R'),(-1,'L')]:
            selected=[i for i in ids if original[i,0]*sign>=0]
            if not selected:continue
            low=bb[2][0];high=bb[2][1]
            shaft_low=low+.004 if stocking else .065
            shaft_high=min(high-.004,.29 if stocking else high-.004)
            heights=np.linspace(shaft_low,shaft_high,30)
            source_centers=[];source_radii=[];target_centers=[];target_radii=[]
            def new_height(z):
                if stocking:return .078+(z-low)*(.29-.078)/(.29-low) if z<.29 else z
                if slug=='renua' and z>.21:return .21+(z-.21)*(.288-.21)/(high-.21)
                return z
            for z in heights:
                center,radius=leg_section(original,edges,z,sign)
                target,skin_radius=leg_section(native,body_edges,new_height(z),sign)
                source_centers.append(center);source_radii.append(radius)
                target_centers.append(target);target_radii.append(skin_radius)
            source_centers=np.array(source_centers);source_radii=np.array(source_radii)
            target_centers=np.array(target_centers);target_radii=np.array(target_radii)
            for i in selected:
                co=original[i];z=co[2];new_z=new_height(z)
                center=np.array([np.interp(z,heights,source_centers[:,a]) for a in range(2)])
                radius=np.array([np.interp(z,heights,source_radii[:,a]) for a in range(2)])
                target=np.array([np.interp(z,heights,target_centers[:,a]) for a in range(2)])
                skin_radius=np.array([np.interp(z,heights,target_radii[:,a]) for a in range(2)])
                scale=np.maximum(1.0,(skin_radius+.005)/np.maximum(radius,.006))
                # Keep the shared stocking crotch in place under the waist panel.
                blend=max(0,min(1,(.32-z)/.03)) if stocking else 1
                xy=co[:2]+blend*(target+(co[:2]-center)*scale-co[:2])
                vertex=obj.data.vertices[i];vertex.co=(xy[0],xy[1],new_z)
                tags.data[i].value=(1 if sign>0 else 2) if stocking else (3 if sign>0 else 4)
                location,_,index,_=tree.find_nearest(vertex.co)
                tri=triangles[index];a,b,c=[body.data.vertices[j].co for j in tri]
                bary=barycentric_transform(location,a,b,c,Vector((1,0,0)),Vector((0,1,0)),Vector((0,0,1)))
                weights={}
                for j,w in zip(tri,bary):
                    for name,value in native_weights[j].items():
                        if name in [f'Thigh.{side}',f'Shin.{side}',f'Foot.{side}'] or (stocking and new_z>.29 and name=='Hips'):
                            weights[name]=weights.get(name,0)+max(0,w)*value
                if boot and new_z<rig.data.bones[f'Foot.{side}'].head_local.z+.005:
                    weights={f'Foot.{side}':1.0}
                if not weights:weights={f'Thigh.{side}':1.0}
                assigned[i]=weights
            print('LEG_FITTED',slug,side,'stocking' if stocking else 'boot',len(selected),flush=True)


def make_rig(body,objects):
    rig=next(o for o in objects if o.type=='ARMATURE')
    apply_transform(rig);rig.name='TSB_Rig'
    rig.animation_data_clear()
    names={'mixamorig:Hips':'Hips','mixamorig:Spine':'Torso','mixamorig:Head':'Head'}
    for source,side in [('Left','R'),('Right','L')]:
        for old,new in [('Arm','UpperArm'),('ForeArm','Forearm'),('Hand','Hand'),('UpLeg','Thigh'),('Leg','Shin'),('Foot','Foot')]:
            names[f'mixamorig:{source}{old}']=f'{new}.{side}'
    for old,new in names.items():
        rig.data.bones[old].name=new
        if old in body.vertex_groups:body.vertex_groups[old].name=new
    bpy.context.view_layer.objects.active=rig
    bpy.ops.object.mode_set(mode='EDIT')
    root=rig.data.edit_bones.new('Root');root.head=(0,0,0);root.tail=(0,0,.06)
    rig.data.edit_bones['Hips'].parent=root
    extras=[('Hair','Head',(0,.05,.78)),('Cape','mixamorig:Spine2',(0,.08,.54)),
            ('Tail','Hips',(0,.08,.35)),('Mount','Root',(0,0,.15)),
            ('Wing.L','mixamorig:Spine2',(-.03,.15,.50)),('Wing.R','mixamorig:Spine2',(.03,.15,.50))]
    for name,parent,head in extras:
        bone=rig.data.edit_bones.new(name);bone.head=head;bone.tail=Vector(head)+Vector((0,0,.06));bone.parent=rig.data.edit_bones[parent]
    bpy.ops.object.mode_set(mode='OBJECT')
    return rig

def bind_parts(body,rig,parts):
    body.data.calc_loop_triangles()
    triangles=[tuple(t.vertices) for t in body.data.loop_triangles]
    tree=BVHTree.FromPolygons([v.co for v in body.data.vertices],triangles,all_triangles=True)
    weights=[{body.vertex_groups[g.group].name:g.weight for g in v.groups} for v in body.data.vertices]
    for kind,obj,assigned in parts:
        groups={bone.name:obj.vertex_groups.new(name=bone.name) for bone in rig.data.bones}
        for v in obj.data.vertices:
            co=v.co
            values=assigned.get(v.index)
            if values is None:
                if kind=='hair':
                    # Keep the scalp and bangs fixed to Head; only the hanging ends may sway.
                    t=max(0,min(1,(.70-co.z)/.40))*.65
                    values={'Head':1-t,'Hair':t}
                elif kind=='clothes' and .22<co.z<.44 and abs(co.x)>.105:
                    values={'Hips':1.0}
                else:
                    location,_,index,_=tree.find_nearest(co)
                    ids=triangles[index];a,b,c=[body.data.vertices[i].co for i in ids]
                    bary=barycentric_transform(location,a,b,c,Vector((1,0,0)),Vector((0,1,0)),Vector((0,0,1)))
                    values={}
                    for i,w in zip(ids,bary):
                        for name,value in weights[i].items():values[name]=values.get(name,0)+max(w,0)*value
            values=dict(sorted(((n,w) for n,w in values.items() if w>1e-5),key=lambda pair:pair[1],reverse=True)[:4])
            total=sum(values.values());assert total>0
            for name,w in values.items():groups[name].add([v.index],w/total,'REPLACE')
        mod=obj.modifiers.new('Tripo skin','ARMATURE');mod.object=rig
    for v in body.data.vertices:
        values=sorted([(g.group,g.weight) for g in v.groups],key=lambda p:p[1],reverse=True)
        kept=values[:4];total=sum(w for i,w in kept);assert total>0
        for i,w in values[4:]:body.vertex_groups[i].remove([v.index])
        for i,w in kept:body.vertex_groups[i].add([v.index],w/total,'REPLACE')

def fit_hair_cap(body,hair,slug):
    """Fit the imported hair volume to the actual skull with a radial cage.

    Both inner and outer layers use the same radial scale. Projecting individual vertices to
    the skull would collapse those layers and produce overlapping polygons. The filled
    Erpin rear volume uses its exterior instead of its interior as the cage reference.
    """
    body.data.calc_loop_triangles();hair.data.calc_loop_triangles()
    scalp=BVHTree.FromPolygons([v.co for v in body.data.vertices],
        [tuple(t.vertices) for t in body.data.loop_triangles],all_triangles=True,epsilon=1e-6)
    cap=BVHTree.FromPolygons([v.co for v in hair.data.vertices],
        [tuple(t.vertices) for t in hair.data.loop_triangles],all_triangles=True,epsilon=1e-6)
    center=Vector((0,-.01,.82))
    fitted=0;scales=[]
    original=[v.co.copy() for v in hair.data.vertices]
    for vertex,co in zip(hair.data.vertices,original):
        lower=.62 if slug=='erpin' else .67
        if co.z<lower:continue
        ray=co-center
        if ray.length<.01:continue
        direction=ray.normalized()
        head_hit,_,_,head_distance=scalp.ray_cast(center,direction,.6)
        hair_hit,_,_,hair_distance=cap.ray_cast(center,direction,.7)
        if slug=='erpin':
            # The long-hair source has a filled rear volume, not a hollow cap.
            # Fit its outside surface; its inner surface crosses the skull center.
            hair_hit,_,_,outside_distance=cap.ray_cast(center+direction*.8,-direction,.8)
            if outside_distance is not None:
                # A ray through a mesh vertex may miss an adjacent outer triangle.
                # Its outer intersection can never be nearer than this vertex.
                hair_distance=max(ray.length,.8-outside_distance)
        if head_hit is None or hair_hit is None or hair_distance<.025:continue
        if slug=='erpin':scale=max(1,min(3.5,(head_distance+.018)/hair_distance))
        else:scale=max(.72,min(1.65,(head_distance+.007)/hair_distance))
        blend=max(0,min(1,(co.z-lower)/(.03 if slug=='erpin' else .08)))
        vertex.co=center+ray*(1+(scale-1)*blend)
        fitted+=1;scales.append(scale)
    print('HAIR_CAP_FITTED',slug,fitted,min(scales),max(scales),flush=True)

def repair_naia_braids(body):
    """The native autorig attached the low braids to the upper arms; restore hair weights."""
    candidates=set()
    for ids in components(body.data):
        bb=bounds(body,ids)
        if bb[1][0]>.06 and bb[2][1]<.66:
            candidates.update(ids)
    head=body.vertex_groups['Head']
    hair=body.vertex_groups.get('Hair') or body.vertex_groups.new(name='Hair')
    for i in candidates:
        vertex=body.data.vertices[i]
        for group in list(vertex.groups):body.vertex_groups[group.group].remove([i])
        t=max(0,min(1,(.72-vertex.co.z)/.30))*.8
        head.add([i],1-t,'REPLACE');hair.add([i],t,'REPLACE')
    print('NAIA_BRAID_WEIGHTS_REPAIRED',len(candidates),flush=True)

def repair_renua_shorts(body,clothes):
    """Keep the native fitted shorts geometry and use the supplied dark fabric."""
    ids=set()
    for group in components(body.data):
        bb=bounds(body,group)
        if .30<bb[2][0]<.41 and bb[2][1]<.44 and len(group)>50:ids.update(group)
    assert len(ids)>500,'missing native fitted shorts'
    material_index=len(body.data.materials);body.data.materials.append(clothes.data.materials[0])
    uv=clothes['shorts_swatch_uv']
    for polygon in body.data.polygons:
        if all(i in ids for i in polygon.vertices):
            polygon.material_index=material_index
            for loop in polygon.loop_indices:body.data.uv_layers.active.data[loop].uv=uv
    print('RENUA_NATIVE_SHORTS_FITTED',len(ids),flush=True)


def remove_covered_body(slug,body):
    if slug=='renua':
        # Tuck the hidden overlap under the non-circular boot rim. Preserve the
        # visible thigh and avoid a cut edge at the opening during knee bends.
        points=np.array([v.co[:] for v in body.data.vertices])
        edges=np.array([e.vertices[:] for e in body.data.edges])
        centers={sign:leg_section(points,edges,.275,sign)[0] for sign in [1,-1]}
        for vertex in body.data.vertices:
            if vertex.co.z<.288 and abs(vertex.co.x)<.14:
                t=max(0,min(1,(.288-vertex.co.z)/.018))
                center=centers[1 if vertex.co.x>0 else -1]
                vertex.co.x=center[0]+(vertex.co.x-center[0])*(1-.20*t)
                vertex.co.y=center[1]+(vertex.co.y-center[1])*(1-.20*t)
    bm=bmesh.new();bm.from_mesh(body.data)
    hidden=[]
    for face in bm.faces:
        co=face.calc_center_median()
        if slug=='renua' and face.material_index>0:continue
        leg=sum(sum(g.weight for g in body.data.vertices[v.index].groups
                    if body.vertex_groups[g.group].name.startswith(('Thigh.','Shin.','Foot.')))
                for v in face.verts)/len(face.verts)>.35
        if leg and co.z<({'naia':.34,'erpin':.32,'renua':.325}[slug]):
            # Keep the visible limb and an overlap well inside the garment opening.
            # Only remove skin fully enclosed by the stocking/boot, never at its rim.
            hidden_below={'naia':-1,'erpin':.28,'renua':.25}[slug]
            if max(v.co.z for v in face.verts)<hidden_below:hidden.append(face)
            continue
        if slug=='naia' and co.z<.08:
            continue
        # Keep continuous native arm skin under the sleeves. The old rectangular
        # deletion cut the forearms in mid-air when the cloth was offset.
        if any(sum(g.weight for g in body.data.vertices[v.index].groups
                   if body.vertex_groups[g.group].name.startswith(('UpperArm.','Forearm.','Hand.')))>0.4
               for v in face.verts):
            continue
        if any(any(body.vertex_groups[g.group].name in ['Hair','Head'] and g.weight>.5
                   for g in body.data.vertices[v.index].groups) for v in face.verts):
            continue
        if slug=='naia':
            covered=(.44<co.z<.575 and abs(co.x)<.12) or (co.z<.31 and co.z>.025) or (.30<co.z<.40 and abs(co.x)<.105) or (.52<co.z<.60 and .105<abs(co.x)<.245)
        elif slug=='erpin':
            covered=((co.z<.095 or .16<co.z<.53) and abs(co.x)<.125) or (.51<co.z<.60 and abs(co.x)<.245)
        else:
            covered=(co.z<.53 and abs(co.x)<.135) or (.51<co.z<.60 and abs(co.x)<.245)
        if covered:hidden.append(face)
    bmesh.ops.delete(bm,geom=hidden,context='FACES');bm.to_mesh(body.data);bm.free()

def animate(rig,index):
    scene=bpy.context.scene
    author.RIG=rig;author.animate(index)
    frames=[]
    for f in range(529):
        scene.frame_set(f)
        frames.append({p.name:(p.rotation_euler.copy(),p.location.copy()) for p in rig.pose.bones})
    rig.animation_data_clear();rig.animation_data_create()
    rig.animation_data.action=bpy.data.actions.new('TSB_Motions_Tripo')
    old_axes=Matrix.Rotation(math.pi/2,3,'X')
    for pb in rig.pose.bones:pb.rotation_mode='QUATERNION'
    for f,poses in enumerate(frames):
        for pb in rig.pose.bones:
            rotation,location=poses[pb.name]
            rest=pb.bone.matrix_local.to_3x3()
            converter=rest.inverted()@old_axes
            delta=converter@rotation.to_matrix()@converter.inverted()
            if pb.name.startswith('UpperArm.'):
                # Native upload is T posed; the authored TSB rest pose has relaxed arms.
                sign=1 if pb.name.endswith('.R') else -1
                bias=rest.inverted()@Matrix.Rotation(sign*.93,3,'Y')@rest
                delta=bias@delta
            if pb.name.startswith('Wing.'):
                start,end=next(((s,e) for s,e in author.CLIPS.values() if s<=f<=e),(f,f+1))
                phase=(f-start)/(end-start)*math.tau
                sign=1 if pb.name.endswith('.R') else -1
                delta=Matrix.Rotation(sign*.08*math.sin(phase),3,'Z')
            pb.rotation_quaternion=delta.to_quaternion()
            pb.location=rest.inverted()@location
            pb.keyframe_insert(data_path='rotation_quaternion',frame=f,group=pb.name)
            pb.keyframe_insert(data_path='location',frame=f,group=pb.name)

def render(scene,camera,path,frame,position=(0,-5,1.05),target=(0,0,1.05)):
    scene.frame_set(frame);camera.location=position
    camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler()
    scene.render.filepath=str(path);bpy.ops.render.render(write_still=True)

def export_weapon(slug,folder):
    weapon,_=import_part(slug,'weapon')
    if slug=='naia':
        hat,_=import_part(slug,'weapon-hat')
        transform_component(hat,range(len(hat.data.vertices)),(.34,.32,.26),(0,-.08,.80))
        bpy.ops.object.select_all(action='DESELECT')
        weapon.select_set(True);hat.select_set(True);bpy.context.view_layer.objects.active=weapon
        bpy.ops.object.join()
    weapon.name='TSB_WeaponMesh'
    # Runtime weapon coordinates: origin at the grip, +Z forward (Blender -Y).
    for v in weapon.data.vertices:
        if slug=='erpin':v.co=(v.co-Vector((0,0,.42)))*1.65
        elif slug=='naia':v.co=(v.co-Vector((0,.24,.22)))*.65
        else:v.co=(v.co-Vector((0,0,.37)))*.75+Vector((0,-.16,.10))
    bpy.ops.object.select_all(action='DESELECT');weapon.select_set(True)
    bpy.context.view_layer.objects.active=weapon
    bpy.ops.export_scene.fbx(filepath=str(folder/f'{slug}-weapon.fbx'),use_selection=True,
        object_types={'MESH'},axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
    weapon.hide_render=True;weapon.hide_viewport=True
    return weapon

def main(slug):
    spec=SPECS[slug];folder=OUT/slug;folder.mkdir(exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    body,objects=import_part(slug,'rigged');rig=make_rig(body,objects)
    if slug=='naia':repair_naia_braids(body)
    parts=[]
    for kind in spec['parts']:
        if kind in ['rigged','weapon','weapon-hat']:continue
        obj,_=import_part(slug,kind)
        assigned=fit(slug,kind,obj)
        if kind=='clothes':fit_sleeves(obj,rig,slug,assigned)
        if kind in ['clothes','sandals']:fit_legs(body,rig,obj,slug,kind,assigned)
        if kind=='hair':fit_hair_cap(body,obj,slug)
        parts.append((kind,obj,assigned))
    if slug=='renua':repair_renua_shorts(body,next(obj for kind,obj,_ in parts if kind=='clothes'))
    bind_parts(body,rig,parts)
    remove_covered_body(slug,body)
    for obj in [body]+[p[1] for p in parts]:
        for v in obj.data.vertices:v.co*=1.85
    bpy.context.view_layer.objects.active=rig;rig.select_set(True)
    bpy.ops.object.mode_set(mode='EDIT')
    positions={b.name:(b.head.copy()*1.85,b.tail.copy()*1.85) for b in rig.data.edit_bones}
    for b in rig.data.edit_bones:b.head,b.tail=positions[b.name]
    bpy.ops.object.mode_set(mode='OBJECT')
    bpy.ops.object.select_all(action='DESELECT')
    for obj in [body]+[p[1] for p in parts]:obj.select_set(True)
    bpy.context.view_layer.objects.active=body;bpy.ops.object.join()
    body.name=f'{slug}_SkinnedMesh';body.parent=rig
    scene,camera=lighting();scene.cycles.samples=12
    scene.render.resolution_x=scene.render.resolution_y=640
    camera.data.ortho_scale=2.6
    scene.render.fps=30;scene.frame_start=0;scene.frame_end=528
    if '--detail-review' not in sys.argv:
        render(scene,camera,folder/'assembled-front.png',0)
        render(scene,camera,folder/'assembled-side.png',0,position=(5,0,1.05))
        render(scene,camera,folder/'assembled-back.png',0,position=(0,5,1.05))
        camera.data.ortho_scale=1.35
        render(scene,camera,folder/'head-front.png',0,position=(0,-5,1.55),target=(0,0,1.55))
        render(scene,camera,folder/'head-side.png',0,position=(5,0,1.55),target=(0,0,1.55))
    camera.data.ortho_scale=2.6
    bpy.ops.wm.save_as_mainfile(filepath=str(folder/f'{slug}-fit.blend'))
    if '--fit-only' in sys.argv:
        if slug=='naia':
            weapon=export_weapon(slug,folder)
            body.hide_render=True;weapon.hide_render=False;weapon.hide_viewport=False
            camera.data.ortho_scale=1.05
            render(scene,camera,folder/'weapon-fit.png',0,position=(2,-3,1.2),target=(0,-.14,.30))
        return
    animate(rig,spec['id'])
    scene['clips']=json.dumps(author.CLIPS)
    scene['selection_gesture']=author.SELECT_GESTURES[spec['id']]
    scene['source']='User-provided Tripo body, clothes, hair and accessories'
    guide=bpy.data.images.load(str(ROOT/'assets/turnarounds'/f'{slug}.png'));guide.pack()
    ref=bpy.data.objects.new('Turnaround_REFERENCE_only',None);scene.collection.objects.link(ref)
    ref.empty_display_type='IMAGE';ref.data=guide;ref.hide_render=True;ref.hide_viewport=True
    export_weapon(slug,folder)
    texture_paths=[]
    for kind in spec['parts']:
        filename=f'{slug}-{kind}-basecolor.jpg'
        source=ROOT/'assets/character-sources/tripo'/slug/kind/f'{slug}-{kind}.fbm'/filename
        shutil.copy2(source,folder/filename)
        img=bpy.data.images[f'{slug}-{kind}-basecolor'];img.filepath=str(folder/filename);img.pack()
        texture_paths.append(f'assets/characters/{filename}')
    # Re-export now that texture paths point to the same directory as the FBXs.
    weapon=bpy.data.objects['TSB_WeaponMesh'];weapon.hide_viewport=False
    bpy.ops.object.select_all(action='DESELECT');weapon.select_set(True);bpy.context.view_layer.objects.active=weapon
    bpy.ops.export_scene.fbx(filepath=str(folder/f'{slug}-weapon.fbx'),use_selection=True,object_types={'MESH'},
        axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
    weapon.hide_viewport=True
    scene.frame_set(0)
    bpy.ops.wm.save_as_mainfile(filepath=str(folder/f'{slug}.blend'))
    export_fbx(body,rig,folder/f'{slug}.fbx')
    render(scene,camera,folder/f'{slug}-preview.png',0,position=(2.8,-5,2.2))
    if '--detail-review' not in sys.argv:
        for name,frame in [('idle',0),('walk',70),('run',103),('attack',168),('ultimate',252),('dance',305),('select',480),('death',453)]:
            render(scene,camera,folder/f'motion-{name}.png',frame)
    camera.data.ortho_scale=.78
    for name,frame in [('idle',0),('attack',168),('dance',305)]:
        scene.frame_set(frame);bpy.context.view_layer.update()
        wrist=rig.pose.bones['Hand.R'].head
        elbow=rig.pose.bones['Forearm.R'].head
        target=wrist.lerp(elbow,.20)
        render(scene,camera,folder/f'sleeve-{name}-front.png',frame,
               position=target+Vector((0,-5,.15)),target=target)
        if name=='idle':
            render(scene,camera,folder/'sleeve-idle-side.png',frame,
                   position=target+Vector((4,-2,1)),target=target)
    camera.data.ortho_scale=1.0
    for name,frame in [('idle',0),('walk',70),('run',103)]:
        render(scene,camera,folder/f'legs-{name}-front.png',frame,position=(0,-5,.33),target=(0,0,.33))
        render(scene,camera,folder/f'legs-{name}-side.png',frame,position=(5,0,.33),target=(0,0,.33))
    camera.data.ortho_scale=1.22
    scene.render.resolution_x=scene.render.resolution_y=256
    render(scene,camera,folder/f'{slug}-portrait.png',0,position=(0,-5,1.50),target=(0,0,1.50))
    record={'id':spec['id'],'slug':slug,'vertices':len(body.data.vertices),'triangles':sum(len(p.vertices)-2 for p in body.data.polygons),
            'bones':len(rig.data.bones),'clips':author.CLIPS,'fps':30,'selection_gesture':author.SELECT_GESTURES[spec['id']],
            'source':'Tripo (user-provided)','source_manifest':f'assets/character-sources/tripo/{slug}/source-files.json',
            'textures':texture_paths,'weapon_model':f'assets/characters/{slug}-weapon.fbx'}
    (folder/'candidate.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    print('ROSTER_CANDIDATE_OK',slug,flush=True)

if __name__=='__main__':
    requested=[s for s in SPECS if s in sys.argv]
    for slug in requested or SPECS:main(slug)
