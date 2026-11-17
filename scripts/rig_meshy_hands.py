"""Add articulated fingers and mesh-derived palm grip frames without replacing bodies."""
import bpy,bmesh,json,math,sys,shutil
import numpy as np
from pathlib import Path
from mathutils import Vector,Matrix,Quaternion
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
OUT=ROOT/'artifacts/meshy-hand-fix';DEST=ROOT/'assets/characters';BACK=OUT/'before'
BACK.mkdir(parents=True,exist_ok=True)
inspection={r['slug']:r for r in json.loads((OUT/'hand-inspection.json').read_text())}
slugs=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else list(inspection)

def unit(x):return x/max(1e-8,np.linalg.norm(x))
def run(slug):
    for suffix in ['.blend','.fbx','-rig.json']:
        p=DEST/f'{slug}{suffix}'
        if not (BACK/p.name).exists():shutil.copy2(p,BACK/p.name)
    bpy.ops.wm.open_mainfile(filepath=str(BACK/f'{slug}.blend'))
    for image in bpy.data.images:
        filename=image.filepath.replace('\\','/').split('/')[-1]
        if filename and (DEST/filename).is_file():image.filepath=str(DEST/filename)
    rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects[f'{slug}_SkinnedMesh'];scene=bpy.context.scene;scene.frame_set(0)
    points=np.array([v.co[:] for v in body.data.vertices]);oldweights=[{body.vertex_groups[g.group].name:g.weight for g in v.groups} for v in body.data.vertices]
    material=body.data.materials[0]
    image=next(n.image for n in material.node_tree.nodes if n.type=='TEX_IMAGE' and n.image.name==material['meshy_base_texture'])
    image.filepath=str(DEST/f'{slug}-meshy-basecolor.png')
    width,height=image.size;pixels=np.array(image.pixels[:],dtype=np.float32).reshape(height,width,4)
    uv=np.array([u.uv[:] for u in body.data.uv_layers.active.data]);xy=np.clip((uv*np.array([width,height])).astype(int),[0,0],[width-1,height-1])
    colors=np.zeros((len(points),3));counts=np.zeros(len(points));indices=np.array([loop.vertex_index for loop in body.data.loops])
    np.add.at(colors,indices,pixels[xy[:,1],xy[:,0],:3]);np.add.at(counts,indices,1);colors/=np.maximum(1,counts[:,None])
    red,green,blue=colors.T;skin=(red>.25)&(green>.17)&(blue>.16)&(red>green*.80)&(red>blue*.78)
    neighbors=[[] for _ in points]
    for edge in body.data.edges:
        a,b=edge.vertices;neighbors[a].append(b);neighbors[b].append(a)
    profiles=[];definitions=[];assignments={};wrists={};hand_surfaces={};basis=body.data.shape_keys.key_blocks['Basis']
    assert np.max(np.abs(points-np.array([v.co[:] for v in basis.data])))<1e-5
    for side in ['L','R']:
        name=f'Hand.{side}';w=np.array(rig.data.bones[name].head_local);elbow=np.array(rig.data.bones[f'Forearm.{side}'].head_local)
        # Cloth cuffs follow the forearm; twisting the palm must not roll sleeves into the grip.
        for i,row in enumerate(oldweights):
            if row.get(name,0)>.001 and not skin[i] and np.linalg.norm(points[i]-w)<.20:
                fixed=dict(row);value=fixed.pop(name);fore=f'Forearm.{side}';fixed[fore]=fixed.get(fore,0)+value;assignments[i]=fixed
        f=unit(w-elbow);weights=np.array([v.get(name,0) for v in oldweights]);delta=points-w
        along=delta@f;mask=(weights>.45)&(along>.01)&(along<.23)&(np.linalg.norm(delta,axis=1)<.26)&skin
        # Automatic hand weights can also include detached hair/ribbons. Keep the
        # connected skin surface nearest the anatomical palm for finger placement.
        remaining=set(int(i) for i in np.flatnonzero(mask));components=[]
        while remaining:
            first=remaining.pop();stack=[first];component=[first]
            while stack:
                for j in neighbors[stack.pop()]:
                    if j in remaining:remaining.remove(j);stack.append(j);component.append(j)
            if len(component)>=12:components.append(component)
        if components:
            anchor=w+f*.075
            component=min(components,key=lambda c:np.min(np.linalg.norm(points[c]-anchor,axis=1)))
            mask[:]=False;mask[component]=True
        hand_surfaces[side]=set(int(i) for i in np.flatnonzero(mask))
        # Meshy sometimes places the hand joint above a long sleeve. Put the wrist
        # at the start of the visible hand, leaving the sleeve on the forearm.
        start=float(np.clip(np.quantile(along[mask],.05)-.008,0,.11))
        if np.quantile(along[mask],.98)<.14:start=0.0
        w=w+f*start;wrists[name]=w.copy();delta=points-w
        sample=delta[mask];projected=sample-np.outer(sample@f,f)
        vals,vecs=np.linalg.eigh(np.cov(projected.T));a=unit(vecs[:,-1]);n=unit(np.cross(f,a))
        length=float(np.quantile(sample@f,.98));length=float(np.clip(length,.085,.19))
        # The thumb protrudes beside the proximal palm, before the long finger tips.
        low=sample[(sample@f<length*.65)&(sample@f>length*.10)]
        high=sample[sample@f>np.quantile(sample@f,.65)]
        middle=float(np.median(high@a));lowx=low@a
        if np.quantile(lowx,.98)-middle < middle-np.quantile(lowx,.02):a=-a
        # Right-hand winding gives the back of the hand; mirror the anatomical left.
        n=unit(np.cross(f,a))*(1 if side=='L' else -1)
        t=delta@f;u=delta@a;v=delta@n
        hi=mask&(t>np.quantile(t[mask],.65))
        low_u,high_u=np.quantile(u[hi],[.05,.95]);width=float(high_u-low_u)
        center_u=float((low_u+high_u)*.5);plane=float(np.median(v[mask]))
        root_t=length*.43
        finger_centers=np.quantile(u[hi],[.85,.62,.38,.15])
        chains=[];bone_names=[]
        for digit,c in zip(['Index','Middle','Ring','Pinky'],finger_centers):
            lane=hi&(np.abs(u-c)<width*.19)
            tip_t=float(np.quantile(t[lane],.98)) if lane.sum()>3 else length
            tip_t=max(tip_t,root_t+.04)
            root=w+f*root_t+a*c+n*plane;tip=w+f*tip_t+a*c+n*plane
            chains.append((digit,root,tip,a*(1 if side=='L' else -1),[1.0,1.25,.9]))
        thumb_u=float(np.quantile(u[mask&(t<length*.65)],.98))
        thumb_root=w+f*(length*.20)+a*(center_u+width*.28)+n*plane
        thumb_tip=w+f*(length*.56)+a*max(thumb_u,high_u+width*.12)+n*plane
        thumb_f=unit(thumb_tip-thumb_root);thumb_axis=unit(np.cross(thumb_f,-n))
        chains.append(('Thumb',thumb_root,thumb_tip,thumb_axis,[.55,.8,.55]))
        for digit,root,tip,axis,angles in chains:
            joints=[root+(tip-root)*fraction for fraction in [0,.37,.73,1]]
            for i in range(3):
                bone=f'{digit}{i+1}.{side}';parent=name if i==0 else f'{digit}{i}.{side}'
                definitions.append((bone,joints[i],joints[i+1],parent,n));bone_names.append(bone)
        radius=max(.010,(length-root_t)/math.pi*.72)
        contact=w+f*(root_t+radius*.45)+a*center_u+n*(plane-radius)
        socket=f'PalmGrip.{side}'
        definitions.append((socket,contact,contact+a*.045,name,-n))
        hand_matrix=rig.data.bones[name].matrix_local
        controls=[]
        for digit,root,tip,axis,angles in chains:
            axis_local=hand_matrix.to_3x3().inverted()@Vector(axis)
            for i,angle in enumerate(angles):controls.append(dict(name=f'{digit}{i+1}.{side}',axis=list(axis_local),angle=angle))
        profiles.append(dict(side=side,hand=name,socket=socket,controls=controls,length=length,radius=radius,
            forward=list(f),across=list(a),dorsal=list(n),contact=list(contact),wrist=list(w)))
        # Include the continuous surface around the skin seed. A texture-color
        # boundary is not a joint boundary: leaving those vertices rigid tears cuffs
        # and finger valleys. Blend neighboring digits instead of hard Voronoi lanes.
        candidates=np.flatnonzero((weights>.001)&(t>-.015)&(t<length*1.3)&(np.abs(v-plane)<.07)&(np.linalg.norm(delta,axis=1)<.27))
        distances=np.min(np.linalg.norm(points[candidates,None,:]-points[mask][None,:,:],axis=2),axis=1)
        surface=candidates[distances<.022]
        hand_surfaces[side]=set(int(i) for i in surface)
        for index in surface:
            ranked=[]
            for chain in chains:
                digit,root,tip,axis,angles=chain;direction=tip-root
                progress=float(np.dot(points[index]-root,direction)/np.dot(direction,direction))
                distance=np.linalg.norm(points[index]-(root+direction*np.clip(progress,0,1)))
                ranked.append((distance,digit,progress))
            ranked.sort();sigma=max(width*.22,.006)
            blends=np.exp(-np.array([r[0]-ranked[0][0] for r in ranked[:2]])**2/(2*sigma*sigma));blends/=sum(blends)
            replacements={}
            for blend,(_,digit,progress) in zip(blends,ranked[:2]):
                influence=float(np.clip((progress+.15)/.35,0,1))
                segment=float(np.clip(progress*3,0,2));low=int(segment);frac=segment-low
                for key,value in [(name,1-influence),(f'{digit}{low+1}.{side}',influence*(1-frac)),(f'{digit}{min(low+2,3)}.{side}',influence*frac)]:
                    replacements[key]=replacements.get(key,0)+float(blend)*value
            row=dict(oldweights[index]);amount=row.pop(name,0)
            for key,value in replacements.items():row[key]=row.get(key,0)+amount*value
            assignments[index]=row
    bpy.ops.object.select_all(action='DESELECT');rig.select_set(True);bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
    for name,position in wrists.items():
        b=rig.data.edit_bones[name];shift=Vector(position)-b.head;b.head+=shift;b.tail+=shift
    for name,head,tail,parent,normal in definitions:
        b=rig.data.edit_bones.new(name);b.head=head;b.tail=tail;b.parent=rig.data.edit_bones[parent];b.use_deform=not name.startswith('PalmGrip')
        b.align_roll(Vector(normal))
    bpy.ops.object.mode_set(mode='OBJECT')
    groups={g.name:g for g in body.vertex_groups}
    for name,*_ in definitions:
        if not name.startswith('PalmGrip'):groups[name]=body.vertex_groups.new(name=name)
    for i,row in assignments.items():
        i=int(i)
        for group_index in [g.group for g in body.data.vertices[i].groups]:body.vertex_groups[group_index].remove([i])
        row=sorted(((n,w) for n,w in row.items() if w>1e-6),key=lambda x:x[1],reverse=True)[:4];total=sum(w for n,w in row)
        for n,w in row:groups[n].add([i],w/total,'REPLACE')
    # Explicit open keys reset fingers every frame before runtime grip/wave posing.
    for profile in profiles:
        for c in profile['controls']:
            p=rig.pose.bones[c['name']];p.rotation_mode='QUATERNION';p.rotation_quaternion=Quaternion()
            for frame in [0,528]:p.keyframe_insert(data_path='rotation_quaternion',frame=frame,group=p.name)
    profiles=json.loads(json.dumps(profiles,default=float))
    scene['hand_rig']=json.dumps(profiles)
    scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(DEST/f'{slug}.blend'))
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and any(m.type=='ARMATURE' and m.object==rig for m in o.modifiers)]
    export_fbx(body,rig,DEST/f'{slug}.fbx',extras)
    record=json.loads((DEST/f'{slug}-rig.json').read_text());record['bones']=len(rig.data.bones);record['hands']=profiles
    (DEST/f'{slug}-rig.json').write_text(json.dumps(record,indent=2))
    (DEST/f'{slug}-hands.json').write_text(json.dumps(profiles,indent=2))
    # Inspect a posed hand around a real-radius grip cylinder from the palm side.
    rig.animation_data_clear()
    for profile in profiles:
        for c in profile['controls']:
            p=rig.pose.bones[c['name']];axis=rig.data.bones[p.name].matrix_local.to_3x3().inverted()@(rig.data.bones[profile['hand']].matrix_local.to_3x3()@Vector(c['axis']))
            p.rotation_quaternion=Quaternion(axis,c['angle'])
    bpy.context.view_layer.update()
    dg=bpy.context.evaluated_depsgraph_get();evaluated=body.evaluated_get(dg)
    isolated=bpy.data.meshes.new_from_object(evaluated,preserve_all_data_layers=True,depsgraph=dg)
    allowed=hand_surfaces['L']
    bm=bmesh.new();bm.from_mesh(isolated);bm.verts.ensure_lookup_table();bmesh.ops.delete(bm,geom=[v for v in bm.verts if v.index not in allowed],context='VERTS');bm.to_mesh(isolated);bm.free()
    for o in bpy.data.objects:
        if o.type=='MESH':o.hide_render=True
    hand_object=bpy.data.objects.new('Isolated posed hand',isolated);bpy.context.collection.objects.link(hand_object)
    profile=profiles[0];center=Vector(profile['contact']);a=Vector(profile['across']);n=Vector(profile['dorsal'])
    bpy.ops.mesh.primitive_cylinder_add(vertices=24,radius=profile['radius']*.72,depth=.20,location=center)
    cylinder=bpy.context.object;cylinder.rotation_euler=a.to_track_quat('Z','Y').to_euler()
    mat=bpy.data.materials.new('Grip inspection red');mat.diffuse_color=(.5,.04,.02,1);cylinder.data.materials.append(mat)
    for o in list(bpy.data.objects):
        if o.type in {'CAMERA','LIGHT'}:bpy.data.objects.remove(o,do_unlink=True)
    scene.render.engine='CYCLES';scene.cycles.samples=16;scene.render.resolution_x=640;scene.render.resolution_y=640;scene.render.resolution_percentage=100;scene.view_settings.view_transform='AgX'
    if scene.world is None:scene.world=bpy.data.worlds.new('Hand review world')
    scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[1].default_value=.8
    for direction,energy in [((1,-2,3),100),((-2,1,2),100)]:
        bpy.ops.object.light_add(type='AREA',location=center+Vector(direction));o=bpy.context.object;o.data.energy=energy;o.data.size=2;o.rotation_euler=(center-o.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=center+n*.7+a*.3);cam=bpy.context.object;cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=.32;scene.camera=cam
    scene.render.filepath=str(OUT/f'{slug}-grip-rig.png');bpy.ops.render.render(write_still=True)
    print('HAND_RIG_BUILT',slug,len(assignments),flush=True)
for slug in slugs:run(slug)
