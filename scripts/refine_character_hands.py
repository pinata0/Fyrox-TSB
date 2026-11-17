"""Retopologize open hands and closed, tapered cuffs in an isolated Blender stage.

Faces, expressions and body vertex coordinates are retained. The original arm
surface is removed by connected skin weights; replacement pieces share its rig.
"""
import bpy, bmesh, json, math, shutil, sys
import numpy as np
from pathlib import Path
from mathutils import Vector, Quaternion

ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
from arm_weight_isolation import isolate_refined_arm_weights, REVISION as ARM_ISOLATION_REVISION
OUT=ROOT/'artifacts/hand-mesh-polish';DEST=OUT/'characters';BACK=OUT/'before'
SLUGS='vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
PALETTES=[('eee5e9','b58763'),('a9d9df','ece5d8'),('f2eaf0','c6a762'),('343546','b49bbb'),
 ('dfe3ef','aebace'),('ede4d6','8d6f60'),('38323f','aaa4b4'),('45364e','bc97ba'),
 ('e6eaf0','485b87'),('302a39','a37d9e'),('f4e7d8','c66a69'),('526c94','e0cead'),
 ('e8d2b8','a76f52'),('e8efec','6d9ea6'),('29364f','8b9bb5')]

def unit(v): return v/max(np.linalg.norm(v),1e-9)
def color(hex_value): return tuple(int(hex_value[i:i+2],16)/255 for i in (0,2,4))
def material(label,rgb):
    hx=''.join(f'{round(float(v)*255):02x}' for v in rgb)
    m=bpy.data.materials.new(f'Color_{hx}_{label}');m.diffuse_color=(*rgb,1);m.use_nodes=True
    bsdf=m.node_tree.nodes['Principled BSDF'];bsdf.inputs['Base Color'].default_value=(*rgb,1)
    bsdf.inputs['Roughness'].default_value=.72
    return m

def mesh_object(name,vertices,faces,rig,weights,mats,indices=None):
    data=bpy.data.meshes.new(name);data.from_pydata(vertices,[],faces);data.update()
    obj=bpy.data.objects.new(name,data);bpy.context.collection.objects.link(obj)
    for m in mats:data.materials.append(m)
    for p in data.polygons:p.use_smooth=True
    if indices:
        for p,idx in zip(data.polygons,indices):p.material_index=idx
    groups={}
    for i,row in enumerate(weights):
        row=sorted(((n,w) for n,w in row.items() if w>1e-5),key=lambda x:x[1],reverse=True)[:4]
        total=sum(w for _,w in row)
        for n,w in row:
            if n not in groups:groups[n]=obj.vertex_groups.new(name=n)
            groups[n].add([i],w/total,'REPLACE')
    obj.parent=rig;mod=obj.modifiers.new('TSB clean skin','ARMATURE');mod.object=rig
    return obj

def ellipsoid(center,radii,axes,parts):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=20,ring_count=12,location=(0,0,0))
    obj=bpy.context.object
    transform=np.column_stack(axes)@np.diag(radii)
    for v in obj.data.vertices:v.co=Vector(center+transform@np.array(v.co))
    parts.append(obj)

def run(slug):
    DEST.mkdir(parents=True,exist_ok=True);BACK.mkdir(parents=True,exist_ok=True)
    for suffix in ['.blend','.fbx','-hands.json','-rig.json','.morphs.json']:
        p=ROOT/'assets/characters'/f'{slug}{suffix}'
        if not (BACK/p.name).exists():shutil.copy2(p,BACK/p.name)
    bpy.ops.wm.open_mainfile(filepath=str(BACK/f'{slug}.blend'))
    for im in bpy.data.images:
        name=Path(im.filepath.replace('\\','/')).name
        if (DEST/name).is_file():im.filepath=str(DEST/name)
    rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects[f'{slug}_SkinnedMesh'];scene=bpy.context.scene
    scene.frame_set(0);rig.data.pose_position='REST'
    profiles=json.loads((BACK/f'{slug}-hands.json').read_text())
    points=np.array([v.co[:] for v in body.data.vertices])
    rows=[{body.vertex_groups[g.group].name:g.weight for g in v.groups} for v in body.data.vertices]
    neighbors=[[] for _ in points]
    for edge in body.data.edges:
        a,b=edge.vertices;neighbors[a].append(b);neighbors[b].append(a)
    protected=set()
    for shape in body.data.shape_keys.key_blocks[1:]:
        delta=np.array([v.co[:] for v in shape.data])-points
        protected.update(int(i) for i in np.flatnonzero(np.linalg.norm(delta,axis=1)>1e-7))
    for _ in range(2):protected.update(j for i in list(protected) for j in neighbors[i])
    protected_mask=np.zeros(len(points),dtype=bool);protected_mask[list(protected)]=True
    removed=set();created=[];definitions=[];new_profiles=[];report=[]
    cloth,trim=[material(f'{slug}_{label}',color(h)) for label,h in zip(['cloth','trim'],PALETTES[SLUGS.index(slug)])]
    skin=material(f'{slug}_skin',(.96,.82,.79))
    # Use the source hand's skin tint, sampled from its original base-color UVs.
    mat=body.data.materials[0];tex=next(n.image for n in mat.node_tree.nodes if n.type=='TEX_IMAGE' and n.image.name==mat['meshy_base_texture'])
    pixels=np.array(tex.pixels[:],dtype=np.float32).reshape(tex.size[1],tex.size[0],4)
    loop_uv=np.array([u.uv[:] for u in body.data.uv_layers.active.data])
    xy=np.clip((loop_uv*np.array(tex.size)).astype(int),[0,0],np.array(tex.size)-1)
    colors=np.zeros((len(points),3));counts=np.zeros(len(points));vi=np.array([l.vertex_index for l in body.data.loops])
    np.add.at(colors,vi,pixels[xy[:,1],xy[:,0],:3]);np.add.at(counts,vi,1);colors/=np.maximum(1,counts[:,None])
    for p in profiles:
        side=p['side'];hand=p['hand'];fore=f'Forearm.{side}';upper=f'UpperArm.{side}'
        w=np.array(p['wrist']);e=np.array(rig.data.bones[fore].head_local);s=np.array(rig.data.bones[upper].head_local)
        f=unit(np.array(p['forward']));a=unit(np.array(p['across']));n=unit(np.array(p['dorsal']))
        # Unified dimensions keep the reconstructed fingers clear of the cuff.
        scale=float(np.clip(p['length']/.145,.80,1.12));length=.140*scale
        width=.076*scale;root_t=.064*scale;finger_radius=.0081*scale
        hand_names={hand}|{c['name'] for c in p['controls']}
        hand_weight=np.array([sum(v for k,v in row.items() if k in hand_names) for row in rows])
        samples=colors[(hand_weight>.5)&(np.linalg.norm(points-w,axis=1)<.20)]
        samples=samples[(samples[:,0]>samples[:,1]*.93)&(samples[:,0]>samples[:,2]*.93)&(samples[:,0]>.35)]
        if len(samples):
            rgb=np.quantile(samples,.65,axis=0)
            # Preserve pale gloves/skin without baking strong texture shadow into the new surface.
            rgb=np.clip(rgb,.52,.98)
            hand_mat=material(f'{slug}_skin_{side}',rgb)
        else:hand_mat=skin
        # Select connected arm surface; detached hair with bad automatic weights is excluded.
        arm_names=hand_names|{fore,upper}
        arm_weight=np.array([sum(v for k,v in row.items() if k in arm_names) for row in rows])
        delta=points-s;line=w-s;along=(delta@line)/max(np.dot(line,line),1e-8)
        near=np.linalg.norm(points-(s+np.clip(along,0,1)[:,None]*line),axis=1)<.24
        eligible=(arm_weight>.48)&near&(points[:,2]<s[2]+.065)&~protected_mask
        remaining=set(int(i) for i in np.flatnonzero(eligible));components=[]
        while remaining:
            first=remaining.pop();stack=[first];component=[first]
            while stack:
                for j in neighbors[stack.pop()]:
                    if j in remaining:remaining.remove(j);component.append(j);stack.append(j)
            if len(component)>10:components.append(component)
        if components:
            selected=min(components,key=lambda c:np.min(np.linalg.norm(points[c]-(w+f*.04),axis=1)))
            removed.update(selected)
        # Open hand consists of a rounded palm, wrist and five separated fingers.
        parts=[]
        ellipsoid(w+f*.030*scale,[.044*scale,.036*scale,.0165*scale],[f,a,n],parts)
        ellipsoid(w-f*.024*scale,[.055*scale,.024*scale,.019*scale],[f,a,n],parts)
        chains=[]
        for digit,u,ratio in zip(['Index','Middle','Ring','Pinky'],[.028,.0093,-.0093,-.028],[.94,1.0,.94,.78]):
            base=w+f*root_t+a*u*scale
            tip=base+f*(.070*scale*ratio)
            ellipsoid((base+tip)*.5,[np.linalg.norm(tip-base)*.5+finger_radius,finger_radius,finger_radius*.91],[f,a,n],parts)
            angles=[.30,.85,.75] if digit=='Pinky' else ([.50,.95,1.05] if digit in ['Index','Ring'] else [.55,1.05,1.10])
            chains.append((digit,base,tip,a*(1 if side=='L' else -1),angles))
        thumb_base=w+f*.035*scale+a*.030*scale-n*.004*scale
        thumb_tip=thumb_base+unit(f*.45+a*.89-n*.02)*(.057*scale)
        tf=unit(thumb_tip-thumb_base);ta=unit(np.cross(tf,n));tn=unit(np.cross(tf,ta))
        ellipsoid((thumb_base+thumb_tip)*.5,[np.linalg.norm(thumb_tip-thumb_base)*.5+.011*scale,.0105*scale,.010*scale],[tf,ta,tn],parts)
        # Oppose the thumb across the palm before flexing it; the open mesh stays
        # separate from the index finger instead of fusing the two finger tips.
        thumb_directions=[tf,unit(f*.20-a*.05-n*.98),unit(f*.60-a*.10-n*.79),unit(f*.85-a*.05-n*.525)]
        thumb_axes=[unit(np.cross(before,after)) for before,after in zip(thumb_directions,thumb_directions[1:])]
        thumb_angles=[math.acos(float(np.clip(np.dot(before,after),-1,1))) for before,after in zip(thumb_directions,thumb_directions[1:])]
        chains.append(('Thumb',thumb_base,thumb_tip,np.array(thumb_axes),thumb_angles))
        bpy.ops.object.select_all(action='DESELECT')
        for obj in parts:obj.select_set(True)
        bpy.context.view_layer.objects.active=parts[0];bpy.ops.object.join();obj=parts[0]
        mod=obj.modifiers.new('Continuous palm and finger webs','REMESH');mod.mode='VOXEL';mod.voxel_size=.0019*scale;mod.use_smooth_shade=True
        bpy.ops.object.modifier_apply(modifier=mod.name)
        mod=obj.modifiers.new('Smooth retopology','SMOOTH');mod.factor=.75;mod.iterations=4;bpy.ops.object.modifier_apply(modifier=mod.name)
        mod=obj.modifiers.new('Hand triangle budget','DECIMATE');mod.ratio=.55;bpy.ops.object.modifier_apply(modifier=mod.name)
        bm=bmesh.new();bm.from_mesh(obj.data)
        boundary_vertices={v for edge in bm.edges if edge.is_boundary for v in edge.verts}
        if boundary_vertices:bmesh.ops.remove_doubles(bm,verts=list(boundary_vertices),dist=.0003)
        boundaries=[edge for edge in bm.edges if edge.is_boundary]
        if boundaries:bmesh.ops.holes_fill(bm,edges=boundaries,sides=0)
        bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(obj.data);bm.free()
        vertices=[v.co[:] for v in obj.data.vertices];faces=[tuple(q.vertices) for q in obj.data.polygons]
        bpy.data.objects.remove(obj,do_unlink=True)
        weights=[]
        for vertex in vertices:
            v=np.array(vertex);t=np.dot(v-w,f);ranked=[]
            for digit,base,tip,axis,angles in chains:
                d=tip-base;pr=float(np.dot(v-base,d)/np.dot(d,d));distance=np.linalg.norm(v-(base+d*np.clip(pr,0,1)))
                ranked.append((distance,digit,pr))
            ranked.sort();sigma=.004*scale
            blends=np.exp(-np.array([r[0]-ranked[0][0] for r in ranked[:2]])**2/(2*sigma*sigma));blends/=sum(blends)
            row={}
            for blend,(_,digit,progress) in zip(blends,ranked[:2]):
                influence=float(np.clip((progress+(.08 if digit=='Thumb' else .18))/.35,0,1))
                if digit!='Thumb':influence*=float(np.clip((t-root_t+.014*scale)/(.024*scale),0,1))
                segment=float(np.clip(progress*3,0,2));lo=int(segment);frac=segment-lo
                for key,value in [(hand,1-influence),(f'{digit}{lo+1}.{side}',influence*(1-frac)),(f'{digit}{min(lo+2,3)}.{side}',influence*frac)]:
                    row[key]=row.get(key,0)+float(blend)*value
            # Skin below the cuff follows the forearm; the wrist bends smoothly.
            wrist_mix=float(np.clip((-t)/(scale*.04),0,1))
            row={k:v*(1-wrist_mix) for k,v in row.items()};row[fore]=wrist_mix
            weights.append(row)
        clean=mesh_object(f'{slug}_RefinedHand.{side}',vertices,faces,rig,weights,[hand_mat]);created.append(clean)
        # Closed quad sleeve with a narrowed, rounded cuff and no visible cut edges.
        vertices=[];faces=[];weights=[];indices=[];rings=[]
        cuff=w-f*.026*scale
        nodes=[(s,.048*scale,upper,0),(s+(e-s)*.2,.058*scale,upper,0),(s+(e-s)*.8,.055*scale,upper,0),
               (e,.053*scale,fore,0),(e+(cuff-e)*.35,.050*scale,fore,0),
               (e+(cuff-e)*.75,.040*scale,fore,0),(cuff-f*.012*scale,.0355*scale,fore,0),
               (cuff-f*.007*scale,.0365*scale,fore,1),(cuff-f*.002*scale,.0355*scale,fore,1),
               (cuff,.0325*scale,fore,1)]
        for i,(center,radius,bone,material_index) in enumerate(nodes):
            forward=unit(nodes[min(i+1,len(nodes)-1)][0]-nodes[max(0,i-1)][0])
            ax=unit(a-forward*np.dot(a,forward));az=unit(np.cross(forward,ax))
            ring=[]
            for j in range(24):
                angle=j*math.tau/24;ring.append(len(vertices));vertices.append(tuple(center+radius*(ax*math.cos(angle)+az*math.sin(angle))))
                if i==3:weights.append({upper:.5,fore:.5})
                else:weights.append({bone:1})
            rings.append(ring)
            if i:
                for j in range(24):faces.append((rings[i-1][j],rings[i-1][(j+1)%24],ring[(j+1)%24],ring[j]));indices.append(material_index)
        faces.extend([tuple(reversed(rings[0])),tuple(rings[-1])]);indices.extend([0,1])
        sleeve=mesh_object(f'{slug}_RefinedSleeve.{side}',vertices,faces,rig,weights,[cloth,trim],indices);created.append(sleeve)
        view_sleeve=sleeve.copy();view_sleeve.data=sleeve.data.copy();view_sleeve.name=f'{slug}_RefinedViewSleeve.{side}'
        bpy.context.collection.objects.link(view_sleeve)
        upper_direction=unit(e-s);upper_length=np.linalg.norm(e-s)
        for vertex in view_sleeve.data.vertices:
            progress=float(np.dot(np.array(vertex.co)-s,upper_direction)/upper_length)
            if progress<1:vertex.co-=Vector(upper_direction*(.38*max(0,1-progress)))
        created.append(view_sleeve)
        if slug=='erpin':
            # Retain Erpin's blue diamond setting on the shortened white cuff.
            center=cuff-f*.037*scale+n*.037*scale
            outer=[center+a*.019*scale,center+f*.026*scale,center-a*.019*scale,center-f*.026*scale]
            inner=[center+a*.013*scale+n*.002,center+f*.019*scale+n*.002,center-a*.013*scale+n*.002,center-f*.019*scale+n*.002]
            back=[v-n*.004 for v in outer];vertices=[tuple(v) for v in outer+inner+back]+[tuple(center+n*.007)]
            faces=[];indices=[]
            for j in range(4):
                k=(j+1)%4;faces.extend([(j,k,4+k,4+j),(8+j,8+k,k,j),(4+j,4+k,12)]);indices.extend([0,0,1])
            faces.append((11,10,9,8));indices.append(0)
            gem=mesh_object(f'{slug}_RefinedEmblem.{side}',vertices,faces,rig,[{fore:1} for _ in vertices],[trim,material('erpin_gem',(.22,.65,.78))],indices);created.append(gem)
        controls=[];hand_matrix=rig.data.bones[hand].matrix_local.to_3x3()
        for digit,base,tip,axis,angles in chains:
            joints=[base+(tip-base)*t for t in [0,.36,.70,1]]
            for i,angle in enumerate(angles):
                name=f'{digit}{i+1}.{side}';definitions.append((name,joints[i],joints[i+1],n))
                control_axis=axis[i] if np.ndim(axis)==2 else axis
                controls.append(dict(name=name,axis=list(hand_matrix.inverted()@Vector(control_axis)),angle=angle))
        contact=w+f*.068*scale-n*.024*scale
        definitions.append((p['socket'],contact,contact+a*.045,-n))
        p.update(controls=controls,length=length,radius=.013*scale,contact=list(contact),refined_mesh=True)
        new_profiles.append(p);report.append(dict(side=side,hand_vertices=len(clean.data.vertices),hand_faces=len(clean.data.polygons),cuff_radius=.0365*scale,cuff_clearance=.026*scale))
    # Keep all original vertex indices and facial shape deltas, remove old arm faces only.
    shape_data={k.name:np.array([v.co[:] for v in k.data]) for k in body.data.shape_keys.key_blocks}
    old=body.data;kept=[poly for poly in old.polygons if not any(i in removed for i in poly.vertices)]
    new=bpy.data.meshes.new(f'{slug}_CleanBody');new.from_pydata(points,[],[tuple(poly.vertices) for poly in kept]);new.update()
    for m in old.materials:new.materials.append(m)
    uvs=new.uv_layers.new(name='UVMap')
    for poly,source in zip(new.polygons,kept):
        poly.material_index=source.material_index;poly.use_smooth=True
        for dst,src in zip(poly.loop_indices,source.loop_indices):uvs.data[dst].uv=old.uv_layers.active.data[src].uv
    border={j for i in removed for j in neighbors[i] if j not in removed}
    bm=bmesh.new();bm.from_mesh(new);bm.verts.ensure_lookup_table()
    boundaries=[edge for edge in bm.edges if edge.is_boundary and all(v.index in border for v in edge.verts)]
    if boundaries:bmesh.ops.holes_fill(bm,edges=boundaries,sides=0)
    bm.to_mesh(new);bm.free()
    assert len(new.vertices)==len(points) and np.max(np.abs(np.array([v.co[:] for v in new.vertices])-points))<1e-7
    body.shape_key_clear();body.data=new
    for name in sorted({name for row in rows for name in row}):
        if name not in body.vertex_groups:body.vertex_groups.new(name=name)
    for name,coords in shape_data.items():
        shape=body.shape_key_add(name=name);shape.data.foreach_set('co',coords.astype(np.float32).ravel());shape.value=0
    for i,row in enumerate(rows):
        for name,value in row.items():body.vertex_groups[name].add([i],value,'REPLACE')
    bpy.ops.object.select_all(action='DESELECT');rig.select_set(True);bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
    for name,head,tail,normal in definitions:
        b=rig.data.edit_bones[name];b.head=head;b.tail=tail;b.align_roll(Vector(normal))
    bpy.ops.object.mode_set(mode='OBJECT');rig.data.pose_position='POSE';scene.frame_set(0)
    record=json.loads((BACK/f'{slug}-rig.json').read_text());record['hands']=new_profiles;record['refined_hands']=report
    record['arm_weight_isolation'] = dict(revision=ARM_ISOLATION_REVISION,
        **isolate_refined_arm_weights(body, rig))
    record['triangles']=sum(len(p.vertices)-2 for p in body.data.polygons)
    (DEST/f'{slug}-hands.json').write_text(json.dumps(new_profiles,indent=2,default=float))
    (DEST/f'{slug}-rig.json').write_text(json.dumps(record,indent=2,default=float))
    scene['hand_rig']=json.dumps(new_profiles,default=float);scene['hand_mesh_revision']='closed-cuff-retopology-v1'
    bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(DEST/f'{slug}.blend'))
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and any(m.type=='ARMATURE' and m.object==rig for m in o.modifiers)]
    export_fbx(body,rig,DEST/f'{slug}.fbx',extras)
    # Inspect the actual closed topology, without cutting triangles out for the render.
    rig.animation_data_clear()
    for bone in rig.pose.bones:bone.rotation_mode='QUATERNION';bone.rotation_quaternion=Quaternion();bone.location=(0,0,0)
    for p in new_profiles:
        for c in p['controls']:
            b=rig.pose.bones[c['name']];axis=rig.pose.bones[p['hand']].matrix.to_3x3()@Vector(c['axis'])
            before=b.matrix.copy();rotation=Quaternion(axis,c['angle']).to_matrix()@before.to_3x3()
            result=rotation.to_4x4();result.translation=before.translation;b.matrix=result;bpy.context.view_layer.update()
    bpy.context.view_layer.update()
    for o in bpy.data.objects:
        if o.type=='MESH':o.hide_render=o not in created or not o.name.endswith('.L') or '_RefinedViewSleeve.' in o.name
        if o.type in {'CAMERA','LIGHT'}:bpy.data.objects.remove(o,do_unlink=True)
    p=new_profiles[0];center=Vector(p['contact']);n=Vector(p['dorsal']);a=Vector(p['across']);f=Vector(p['forward'])
    bpy.ops.mesh.primitive_cylinder_add(vertices=32,radius=p['radius'],depth=.23,location=center)
    obj=bpy.context.object;obj.rotation_euler=a.to_track_quat('Z','Y').to_euler();obj.data.materials.append(material('test_handle',(.28,.12,.045)))
    scene.render.engine='CYCLES';scene.cycles.samples=20;scene.render.resolution_x=800;scene.render.resolution_y=650;scene.render.resolution_percentage=100;scene.view_settings.view_transform='AgX'
    if scene.world is None:scene.world=bpy.data.worlds.new('Hand refinement world')
    scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[1].default_value=.5
    for offset,energy in [((1,-2,3),85),((-2,1,2),65)]:
        bpy.ops.object.light_add(type='AREA',location=center+Vector(offset));light=bpy.context.object;light.data.energy=energy;light.data.size=2;light.rotation_euler=(center-light.location).to_track_quat('-Z','Y').to_euler()
    for name,view in [('palm',-n*.65+a*.24+f*.16),('back',n*.65+a*.24+f*.12)]:
        bpy.ops.object.camera_add(location=center+view);cam=bpy.context.object;cam.rotation_euler=(center-f*.035-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=.31;scene.camera=cam
        scene.render.filepath=str(OUT/f'{slug}-refined-{name}.png');bpy.ops.render.render(write_still=True);bpy.data.objects.remove(cam,do_unlink=True)
    print('REFINED_HAND_MESH_OK',slug,report,flush=True)

selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS
for slug in selected:run(slug)
