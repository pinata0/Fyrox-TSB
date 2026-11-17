"""Build staged game rigs from preserved Meshy sources and existing TSB motion.

Run with Blender --background --python-exit-code 1 --python this_file -- [slugs].
The existing runtime assets are untouched until publish_meshy_assets.py runs.
"""
import bpy
import bmesh
import json
import math
import shutil
import sys
import numpy as np
from pathlib import Path
from mathutils import Vector, Quaternion
from mathutils.bvhtree import BVHTree

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
from build_characters import CLIPS, SLUGS
OUT=ROOT/'artifacts/meshy-import'
STAGE=OUT/'characters'
STAGE.mkdir(parents=True,exist_ok=True)
# Inspected native joints: spine, head, negative-X arm, positive-X arm, legs.
MAPS={
 'picora':(5,17,(25,24,23),(21,20,19),(15,14,13),(10,9,8)),
 'komi':(5,24,(28,27,26),(38,37,36),(11,10,9),(17,16,15)),
 'bella':(8,18,(22,21,20),(27,26,25),(12,11,10),(16,15,14)),
 'vivi':(5,18,(22,21,20),(27,26,25),(10,9,8),(15,14,13)),
 'shion':(17,28,(21,20,19),(25,24,23),(7,6,5),(13,12,11)),
 'id':(5,18,(22,21,20),(27,26,25),(10,9,8),(15,14,13)),
 'hilde':(15,18,(33,32,31),(25,24,23),(6,5,4),(11,10,9)),
 'mayo':(5,28,(24,23,22),(19,18,17),(15,14,13),(10,9,8)),
 'ui':(6,29,(20,19,18),(25,24,23),(11,10,9),(16,15,14)),
 'aya':(5,18,(27,26,25),(22,21,20),(10,9,8),(15,14,13)),
 'erpin':(6,23,(27,26,25),(32,31,30),(16,15,14),(11,10,9)),
 'naia':(5,28,(24,23,22),(19,18,17),(15,14,13),(10,9,8)),
 'chloe':(15,28,(24,23,22),(19,18,17),(6,5,4),(11,10,9)),
 'butter':(5,18,(22,21,20),(27,26,25),(10,9,8),(15,14,13)),
 'renua':(6,25,(18,29,37),(21,33,41),(11,10,9),(16,15,14)),
}
# Facial landmarks in source front render pixels (440x520, ortho 3.132).
LANDMARKS={
 'picora':(220,122,140,24,11), 'komi':(210,110,129,23,12),
 'bella':(233,97,112,18,8), 'vivi':(220,120,134,19,9),
 'shion':(200,133,151,22,9), 'id':(220,126,139,20,6),
 'hilde':(219,135,152,22,8), 'mayo':(220,129,144,19,10),
 'ui':(213,141,156,20,11), 'aya':(220,104,119,18,7),
 'erpin':(220,125,142,18,10), 'naia':(215,125,140,19,9),
 'chloe':(217,137,153,21,13), 'butter':(220,116,132,18,11),
 'renua':(220,124,138,18,8),
}

def native(i):return f'Bone_{i:03d}'
def smooth(x):return np.clip(x,0,1)**2*(3-2*np.clip(x,0,1))

def capture_motion(slug):
    backup=OUT/'before/assets/characters';backup.mkdir(parents=True,exist_ok=True)
    for suffix in ['.blend','.fbx']:
        p=ROOT/f'assets/characters/{slug}{suffix}'
        if not (backup/p.name).exists():shutil.copy2(p,backup/p.name)
    manifest=ROOT/'assets/characters/manifest.json'
    if not (backup/'manifest.json').exists():shutil.copy2(manifest,backup/'manifest.json')
    bpy.ops.wm.open_mainfile(filepath=str(backup/f'{slug}.blend'))
    rig=bpy.data.objects['TSB_Rig'];scene=bpy.context.scene
    frames=[]
    for f in range(529):
        scene.frame_set(f)
        frames.append({p.name:(p.matrix.to_quaternion().copy(),p.matrix.translation.copy()) for p in rig.pose.bones})
    return frames

def setup_scene(mesh,path,frame=0,face=False):
    scene=bpy.context.scene;scene.frame_set(frame)
    scene.render.engine='CYCLES';scene.cycles.samples=12
    scene.render.resolution_x=600;scene.render.resolution_y=600;scene.render.resolution_percentage=100
    scene.view_settings.view_transform='Standard'
    if scene.world is None:scene.world=bpy.data.worlds.new('ReviewWorld')
    scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.24,.26,.30,1)
    scene.world.node_tree.nodes['Background'].inputs[1].default_value=.8
    center=Vector((0,0,1.28 if face else .93))
    for o in list(bpy.data.objects):
        if o.type in {'CAMERA','LIGHT'}:bpy.data.objects.remove(o,do_unlink=True)
    for loc,energy,size in [((3,-4,5),450,5),((-3,-2,3),250,4),((0,3,4),300,3)]:
        bpy.ops.object.light_add(type='AREA',location=loc);o=bpy.context.object;o.data.energy=energy;o.data.size=size
        o.rotation_euler=(center-o.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=center+Vector((0,-5,0)));camera=bpy.context.object
    camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.type='ORTHO';camera.data.ortho_scale=.85 if face else 2.15
    scene.camera=camera;scene.render.filepath=str(path);bpy.ops.render.render(write_still=True)

def build(slug):
    frames=capture_motion(slug)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(ROOT/f'assets/character-sources/meshy/{slug}/{slug}-source.glb'))
    bpy.context.view_layer.update()
    mesh=next(o for o in bpy.data.objects if o.type=='MESH' and o.name.startswith('output'))
    old=next(o for o in bpy.data.objects if o.type=='ARMATURE')
    # Weld glTF's UV-split duplicate points; UVs remain stored per face corner.
    bm=bmesh.new();bm.from_mesh(mesh.data);bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.000001);bm.to_mesh(mesh.data);bm.free()
    bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True);bpy.context.view_layer.objects.active=mesh
    reduction=mesh.modifiers.new('Runtime triangle budget','DECIMATE');reduction.ratio=min(1,40000/max(1,len(mesh.data.polygons)))
    bpy.ops.object.modifier_apply(modifier=reduction.name)
    coords=np.array([mesh.matrix_world@v.co for v in mesh.data.vertices])
    offset=np.array([old.data.bones[native(0)].head_local.x,old.data.bones[native(0)].head_local.y,coords[:,2].min()])
    scale=1.85/np.ptp(coords[:,2]);coords=(coords-offset)*scale
    torso,head,al,ar,ll,lr=MAPS[slug]
    names={native(1):'Hips',native(torso):'Torso',native(head):'Head'}
    for side,arms,legs in [('L',al,ll),('R',ar,lr)]:
        names.update({native(i):f'{n}.{side}' for n,i in zip(['UpperArm','Forearm','Hand'],arms)})
        names.update({native(i):f'{n}.{side}' for n,i in zip(['Thigh','Shin','Foot'],legs)})
    positions={new:(np.array(old.matrix_world@old.data.bones[n].head_local)-offset)*scale for n,new in names.items()}
    positions['Hips']=(positions['Thigh.L']+positions['Thigh.R'])*.5+np.array([0,0,.03])
    # Collapse unreliable auto-generated helper/facial chains into anatomical controls.
    group_mapping={}
    for group in mesh.vertex_groups:
        b=old.data.bones.get(group.name)
        while b and b.name not in names:b=b.parent
        group_mapping[group.index]=names[b.name] if b else 'Hips'
    weights=[]
    for v,co in zip(mesh.data.vertices,coords):
        w={}
        for g in v.groups:
            n=group_mapping[g.group];w[n]=w.get(n,0)+g.weight
        # Hair/head vertices must not inherit shoulder/hand errors in automatic skins.
        if co[2]>positions['Head'][2]+.10:w={'Head':1.0}
        total=sum(w.values());weights.append({n:a/total for n,a in w.items()} if total else {'Head':1.})
    mesh.parent=None;mesh.matrix_world.identity();mesh.modifiers.clear();mesh.vertex_groups.clear()
    mesh.data.vertices.foreach_set('co',coords.astype(np.float32).ravel())
    mesh.name=f'{slug}_SkinnedMesh';mesh.data.name=f'{slug}_MeshyMesh'
    for o in list(bpy.data.objects):
        if o!=mesh:bpy.data.objects.remove(o,do_unlink=True)
    arm=bpy.data.armatures.new(f'{slug}_Rig');rig=bpy.data.objects.new('TSB_Rig',arm);bpy.context.collection.objects.link(rig)
    bpy.ops.object.select_all(action='DESELECT');rig.select_set(True);bpy.context.view_layer.objects.active=rig;bpy.ops.object.mode_set(mode='EDIT')
    parents={'Root':None,'Hips':'Root','Torso':'Hips','Head':'Torso','Hair':'Head','Cape':'Torso','Tail':'Hips','Mount':'Root'}
    positions.update(Root=np.zeros(3),Hair=positions['Head']+[0,.1,.2],Cape=positions['Torso']+[0,.12,0],Tail=positions['Hips']+[0,.15,0],Mount=np.array([0,0,.3]))
    for side in ['L','R']:
        for chain,parent in [(['UpperArm','Forearm','Hand'],'Torso'),(['Thigh','Shin','Foot'],'Hips')]:
            for n in chain:parents[f'{n}.{side}']=parent;parent=f'{n}.{side}'
    for name,parent in parents.items():
        b=arm.edit_bones.new(name);b.head=positions[name];b.tail=b.head+Vector((0,0,.10));b.parent=arm.edit_bones.get(parent) if parent else None
    bpy.ops.object.mode_set(mode='OBJECT')
    groups={n:mesh.vertex_groups.new(name=n) for n in parents}
    for i,w in enumerate(weights):
        p=coords[i]
        # Low amplitude secondary sway only in rear hanging hair, away from the face.
        h=w.get('Head',0)
        if h>.8 and p[1]>.09 and p[2]<positions['Head'][2]+.1:
            t=min(.55,max(0,(positions['Head'][2]+.1-p[2])/.65));w['Head']-=h*t;w['Hair']=h*t
        w=dict(sorted(((n,a) for n,a in w.items() if a>1e-6),key=lambda pair:pair[1],reverse=True)[:4]);total=sum(w.values())
        for n,a in w.items():groups[n].add([i],a/total,'REPLACE')
    mod=mesh.modifiers.new('TSB skin','ARMATURE');mod.object=rig
    mesh.parent=rig
    for m in mesh.data.materials:
        m.name=f'{slug}_meshy_body';bsdf=m.node_tree.nodes.get('Principled BSDF')
        bsdf.inputs['Metallic'].default_value=0;bsdf.inputs['Roughness'].default_value=.82
        for socket in ['Metallic','Roughness','Normal']:
            for link in list(bsdf.inputs[socket].links):m.node_tree.links.remove(link)
        for node in list(m.node_tree.nodes):
            if node.type=='TEX_IMAGE' and node.image:
                if not any(l.to_socket==bsdf.inputs['Base Color'] for l in node.outputs['Color'].links):
                    m.node_tree.nodes.remove(node);continue
                img=node.image;img.scale(2048,2048);img.filepath_raw=str(STAGE/f'{slug}-meshy-basecolor.png');img.file_format='PNG';img.save();img.pack()
    # Sample existing authored global motion relative to its idle pose. New A-pose is retained.
    rig.animation_data_create();rig.animation_data.action=bpy.data.actions.new(f'{slug}_TSB_Motions')
    rest={p.name:p.bone.matrix_local.to_quaternion() for p in rig.pose.bones}
    for p in rig.pose.bones:p.rotation_mode='QUATERNION'
    for f,source in enumerate(frames):
        for p in rig.pose.bones:
            q=source.get(p.name,frames[0].get(p.name,(Quaternion(),Vector())))[0]
            base=frames[0].get(p.name,(Quaternion(),Vector()))[0]
            delta=q@base.inverted()
            if p.parent:
                pn=p.parent.name;pq=source.get(pn,(Quaternion(),Vector()))[0]@frames[0].get(pn,(Quaternion(),Vector()))[0].inverted()
                p.rotation_quaternion=rest[p.name].inverted()@pq.inverted()@delta@rest[p.name]
            else:p.rotation_quaternion=rest[p.name].inverted()@delta@rest[p.name]
            p.location=(0,0,0)
            if p.name=='Hips':
                shift=source['Hips'][1]-frames[0]['Hips'][1]
                p.location=p.bone.matrix_local.to_3x3().inverted()@shift
            p.keyframe_insert(data_path='rotation_quaternion',frame=f,group=p.name)
            p.keyframe_insert(data_path='location',frame=f,group=p.name)
    for layer in rig.animation_data.action.layers:
        for strip in layer.strips:
            for bag in strip.channelbags:
                for curve in bag.fcurves:
                    for key in curve.keyframe_points:key.interpolation='LINEAR'
    face=add_face(mesh,slug,coords,offset,scale)
    blink_links=paint_blink_textures(mesh,face,slug)
    scene=bpy.context.scene;scene.render.fps=30;scene.frame_start=0;scene.frame_end=528;scene.frame_set(0)
    scene['clips']=json.dumps(CLIPS);scene['source']='Meshy user-supplied 2026-11-15'
    reference=bpy.data.objects.new('Turnaround_REFERENCE_only',None);scene.collection.objects.link(reference)
    reference.empty_display_type='IMAGE';reference.data=bpy.data.images.load(str(ROOT/f'assets/turnarounds/{slug}.png'));reference.data.pack();reference.hide_render=True
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    material=mesh.data.materials[0];bsdf=material.node_tree.nodes.get('Principled BSDF')
    material.node_tree.links.new(blink_links[0],bsdf.inputs['Base Color'])
    export_fbx(mesh,rig,STAGE/f'{slug}.fbx')
    material.node_tree.links.new(blink_links[1],bsdf.inputs['Base Color'])
    record=dict(slug=slug,vertices=len(mesh.data.vertices),triangles=sum(len(p.vertices)-2 for p in mesh.data.polygons),bones=len(rig.data.bones),clips=CLIPS,fps=30,face=face,source_scale=scale,source_offset=list(offset))
    (STAGE/f'{slug}-rig.json').write_text(json.dumps(record,indent=2,default=float))
    setup_scene(mesh,OUT/f'{slug}-idle.png')
    for name in ['eyeBlink_L','eyeBlink_R','mouthSmile_L','mouthSmile_R']:mesh.data.shape_keys.key_blocks[name].value=1
    setup_scene(mesh,OUT/f'{slug}-expression.png',face=True)
    print('MESHY_CHARACTER_OK',slug,flush=True)

def add_face(mesh,slug,points,offset,scale):
    px,ey,mo,ex,eh=LANDMARKS[slug];unit=3.132/520
    cx=((px-220)*unit-offset[0])*scale
    ez=(.35+(260-ey)*unit-offset[2])*scale
    mz=(.35+(260-mo)*unit-offset[2])*scale
    ex=ex*unit*scale;eh=eh*unit*scale
    x,y,z=points.T
    # Restrict to the visible front shell; no scalp, rear hair or body deformation.
    near=(np.abs(x-cx)<.35)&(np.abs(z-ez)<.2)
    front=float(np.quantile(y[near],.10));depth=smooth((front+.16-y)/.06)
    basis=mesh.shape_key_add(name='Basis',from_mix=False);basis.value=0
    changes={}
    def target(name,delta):
        key=mesh.shape_key_add(name=name,from_mix=False);key.slider_min=0;key.slider_max=1;key.value=0
        key.data.foreach_set('co',(points+delta).astype(np.float32).ravel())
        changes[name]=int(np.count_nonzero(np.linalg.norm(delta,axis=1)>1e-6))
    def mask(x0,z0,rx,rz):return np.exp(-((x-x0)/rx)**4-((z-z0)/rz)**4)*depth
    for side,sign in [('L',-1),('R',1)]:
        # Meshy paints the eye onto the continuous face surface. A flat inner mask
        # closes that illustrated lid; the surrounding skin eases into the fold.
        rx=ex*.73
        eye=(1-smooth((np.abs(x-(cx+sign*ex))-rx*.85)/(rx*.45)))*(1-smooth((np.abs(z-ez)-eh*.95)/(eh*.55)))*depth
        brow=mask(cx+sign*ex,ez+eh*1.8,.095,eh*.7)
        mouth=mask(cx+sign*.047,mz,.065,.045)
        cheek=mask(cx+sign*(ex+.04),mz+.016,.12,.07)
        # Cheek channels explicitly pin the complete eye region.
        protected=(np.abs(np.abs(x-cx)-ex)<.11)&(np.abs(z-ez)<eh*1.4)
        cheek*=~protected
        for name in ['eyeBlink','eyeSquint','eyeWide','browUp','browDown','mouthSmile','mouthFrown','cheekPuff','cheekStretch','cheekSquash']:
            d=np.zeros_like(points)
            if name.startswith('eye'):
                factor={'eyeBlink':-.035,'eyeSquint':-.12,'eyeWide':.12}[name]
                d[:,2]=(z-ez)*factor*eye
            elif name.startswith('brow'):d[:,2]=(.025 if name=='browUp' else -.018)*brow
            elif name.startswith('mouth'):
                d[:,2]=(.022 if name=='mouthSmile' else -.018)*mouth;d[:,0]=sign*.008*mouth
            elif name=='cheekPuff':d[:,1]=-.03*cheek
            elif name=='cheekStretch':d[:,0]=sign*.12*cheek
            else:d[:,0]=-sign*.025*cheek;d[:,1]=.015*cheek
            target(f'{name}_{side}',d)
    mouth=mask(cx,mz,.085,.055)
    for name in ['jawOpen','mouthPucker','mouthWide']:
        d=np.zeros_like(points)
        if name=='jawOpen':d[:,2]=(z-mz)*.7*mouth-.016*mouth
        elif name=='mouthPucker':d[:,0]=-(x-cx)*.3*mouth;d[:,1]=-.02*mouth
        else:d[:,0]=(x-cx)*.4*mouth
        target(name,d)
    assert all(n>4 for n in changes.values()),(slug,changes)
    return dict(center_x=cx,eye_y=ez,eye_x=ex,mouth_y=mz,front=-front,eye_half_height=eh,morph_vertices=changes)

def paint_blink_textures(mesh,face,slug,build_nodes=True,natural=False):
    """Bake closed illustrated lids into the original UV atlas."""
    mat=mesh.data.materials[0];nodes=mat.node_tree.nodes;links=mat.node_tree.links;bsdf=nodes.get('Principled BSDF')
    original=next(n for n in nodes if n.type=='TEX_IMAGE');image=original.image;w,h=image.size
    mat['meshy_base_texture']=image.name
    pixels=np.array(image.pixels[:],dtype=np.float32).reshape(h,w,4)
    mesh.data.calc_loop_triangles();tris=list(mesh.data.loop_triangles)
    coords=np.array([v.co[:] for v in mesh.data.vertices]);uvs=np.array([u.uv[:] for u in mesh.data.uv_layers.active.data])
    tree=BVHTree.FromPolygons([v.co for v in mesh.data.vertices],[t.vertices[:] for t in tris],all_triangles=True)
    cx=face['center_x'];ez=face['eye_y'];ex=face['eye_x'];eh=face['eye_half_height']
    skin_samples=[]
    for z in [ez-.015,ez-.025,ez-.035]:
        hit,_,index,_=tree.ray_cast(Vector((cx,-3,z)),Vector((0,1,0)))
        if hit:
            tri=tris[index];p=coords[list(tri.vertices)];uv=uvs[list(tri.loops)]
            a=np.linalg.lstsq(np.vstack([p.T,np.ones(3)]),np.append(np.array(hit),1),rcond=None)[0]
            uv=a@uv;skin_samples.append(pixels[min(h-1,int(uv[1]*h)),min(w-1,int(uv[0]*w)),:3])
    skin=np.median(skin_samples,axis=0)
    if skin.mean()<.4:skin=np.array([.97,.83,.78])
    outputs={side:pixels.copy() for side in (['L','R','closed','half'] if natural else ['L','R','closed'])}
    coverage={side:np.zeros((h,w),dtype=np.float32) for side in outputs}
    if natural:
        from natural_eyelids import sample_lids,lid_samples
        from blink_surfaces import fit_surfaces,footprint,protected_vertices
        lid_profiles=sample_lids(tree,tris,coords,uvs,pixels,face,skip=('R',) if slug=='aya' else ())
        accessories=protected_vertices(mesh)
        fit_surfaces(coords,lid_profiles,face,accessories)
    for tri in tris:
        p=coords[list(tri.vertices)]
        if p[:,2].max()<ez-eh*1.7 or p[:,2].min()>ez+eh*1.7 or p[:,1].min()>-face['front']+.10:continue
        uv=uvs[list(tri.loops)]*np.array([w,h]);low=np.maximum(0,np.floor(uv.min(0)).astype(int));high=np.minimum([w-1,h-1],np.ceil(uv.max(0)).astype(int))
        if np.prod(high-low+1)>100000:continue
        matrix=np.vstack([uv.T,np.ones(3)])
        if abs(np.linalg.det(matrix))<1e-8:continue
        xx,yy=np.meshgrid(np.arange(low[0],high[0]+1),np.arange(low[1],high[1]+1));at=np.stack([xx.ravel()+.5,yy.ravel()+.5,np.ones(xx.size)])
        bary=np.linalg.solve(matrix,at).T;inside=np.all(bary>=-.001,axis=1)
        pos=bary@p;ix=xx.ravel();iy=yy.ravel()
        for side,sign in [('L',-1),('R',1)]:
            if slug=='aya' and side=='R':continue
            dx=pos[:,0]-(cx+sign*ex);dz=pos[:,2]-ez;rx=ex*.83;rz=eh*1.6
            radius=(dx/rx)**2+(dz/rz)**2
            alpha=(1-smooth((radius-.72)/.28))*inside*smooth((-face['front']+.12-pos[:,1])/.06)
            if slug=='chloe':
                # Include the full upper lash wing/crease, while preserving blonde bangs.
                rx=ex*1.26;rz=eh*1.60
                radius=np.where(dz<0,(dx/(ex*.90))**2+(dz/(eh*1.30))**2,(dx/rx)**4+((dz-eh*.10)/rz)**4)
                base=pixels[iy,ix,:3]
                # Only preserve yellow hair at the edge; red/yellow iris pixels
                # in the eye interior must disappear with the white and lashes.
                hair=(base[:,0]>.45)&(base[:,1]>base[:,0]*.60)&((dz>eh*.8)|(np.abs(dx)>ex*.98))
                skin_or_ink=1-hair*(1-smooth((base[:,2]/np.maximum(base[:,1],.001)-.76)/.12))
                alpha=(1-smooth((radius-.72)/.28))*inside*skin_or_ink*smooth((-face['front']+.12-pos[:,1])/.06)
                # The cheek heart and pink blush sit immediately below the left eye.
                heart=((pos[:,0]-(cx-ex-.055))/.046)**2+((pos[:,2]-(face['mouth_y']+.004))/.035)**2
                alpha*=smooth((heart-.85)/.15)
                blush=(base[:,2]>base[:,1]*1.015)&(base[:,1]<base[:,0]*.90)&(dz<-eh*.65)
                alpha*=~blush
            if natural:
                # Move the whole sampled upper lash, including its outer wing.
                # The older elliptical patch left the wing at its original height.
                profile=lid_profiles[side]
                upper=np.interp(pos[:,0],profile['x'],profile['open'])
                width=np.interp(pos[:,0],profile['x'],profile['source_thickness'])
                surface=np.interp(pos[:,0],profile['x'],profile['depth'])
                lo,hi=profile['span']
                wing=smooth((pos[:,0]-lo+.004)/.006)*smooth((hi+.004-pos[:,0])/.006)
                wing*=1-smooth((np.abs(pos[:,2]-upper)-width*.55-.002)/.005)
                wing*=1-smooth((np.abs(pos[:,1]-surface)-.018)/.018)
                alpha=np.maximum(alpha,wing*inside)
                # Cover the entire outer-corner triangle, not only its upper
                # edge. Its old position must disappear together with the lid.
                # Extend around the upper/outer corner only. The lower iris is
                # already covered by the original rounded eye mask; extending
                # a rectangle into the cheek would cut square holes in blush.
                corner_cover=footprint(profile,pos)*smooth((dz+eh*.60)/.020)
                alpha=np.maximum(alpha,corner_cover*inside)
                # The downward tip of a triangular lash can extend past the
                # upper-band sampler's span. Clear that dark tip at the source
                # eye depth, without painting the adjacent light hair/skin.
                tip=smooth((pos[:,0]-lo+.030)/.008)*smooth((hi+.030-pos[:,0])/.008)
                tip*=smooth((dz+eh*.75)/.012)*smooth((upper+width*.65+.006-pos[:,2])/.008)
                tip*=1-smooth((np.abs(pos[:,1]-surface)-.030)/.020)
                tip*=1-smooth((pixels[iy,ix,:3].max(axis=1)-.38)/.20)
                tip*=smooth((sign*dx-ex*.50)/.025)
                if slug in ['picora','hilde']:alpha=np.maximum(alpha,tip*inside)
                # Cheek paint belongs to the cheek, including Picora's heart and
                # Ui's spirals. Only protect it outside the iris, below the lid.
                base=pixels[iy,ix,:3]
                pink=(base[:,0]>.55)&(base[:,1]<base[:,0]*.86)&(base[:,2]>base[:,1]*1.015)
                cheek=(sign*(pos[:,0]-(cx+sign*ex))>(hi-lo)*.33)&(dz<-eh*.60)
                alpha*=~(pink&cheek)
                if slug=='chloe':
                    alpha*=smooth((heart-.85)/.15)
                    alpha*=~blush
                alpha*=1-smooth((bary@accessories[list(tri.vertices)]-.05)/.80)
            active=alpha>0
            if not active.any():continue
            base=pixels[iy,ix,:3];color=base*(1-alpha[:,None])+skin*alpha[:,None]
            line_z=-.009+.014*(dx/rx)**2
            if slug=='chloe':line_z=-.009+.026*(dx/rx)**2
            ink=(1-smooth((np.abs(dz-line_z)-.0015)/.0025))*(1-smooth((np.abs(dx)/rx-.76)/.12))*alpha
            ink_color=np.array([.17,.12,.14])
            if natural:
                profile=lid_profiles[side]
                closed_y,thick,ink_color,span=lid_samples(profile,pos[:,0],1.0)
                ink=(1-smooth((np.abs(pos[:,2]-closed_y)-thick*.5)/.0015))*span*alpha
                half_y,half_thick,half_ink,half_span=lid_samples(profile,pos[:,0],.52)
                half_alpha=alpha*smooth((pos[:,2]-half_y+.008)/.014)
                lo,hi=profile['span'];corner=np.abs(pos[:,0]-(lo+hi)*.5)/((hi-lo)*.5)
                half_alpha=np.maximum(half_alpha,alpha*smooth((corner-.78)/.16))
                half_color=base*(1-half_alpha[:,None])+skin*half_alpha[:,None]
                half_line=(1-smooth((np.abs(pos[:,2]-half_y)-half_thick*.5)/.0015))*half_span*alpha
                half_color=half_color*(1-half_line[:,None])+half_ink*half_line[:,None]
                half_active=(half_alpha>0)|(half_line>0)
                outputs['half'][iy[half_active],ix[half_active],:3]=half_color[half_active]
                coverage['half'][iy[half_active],ix[half_active]]=np.maximum(coverage['half'][iy[half_active],ix[half_active]],np.maximum(half_alpha,half_line)[half_active])
            color=color*(1-ink[:,None])+ink_color*ink[:,None]
            for key in [side,'closed']:
                outputs[key][iy[active],ix[active],:3]=color[active]
                coverage[key][iy[active],ix[active]]=np.maximum(coverage[key][iy[active],ix[active]],alpha[active])
    # Pad painted UV islands so bilinear filtering/mipmaps cannot sample the old
    # iris from unpainted texels between atlas triangles.
    for key in outputs:
        mask=coverage[key]>.85
        for _ in range(3):
            expanded=mask.copy()
            for source,dest in [((slice(1,None),slice(None)),(slice(None,-1),slice(None))),
                                ((slice(None,-1),slice(None)),(slice(1,None),slice(None))),
                                ((slice(None),slice(1,None)),(slice(None),slice(None,-1))),
                                ((slice(None),slice(None,-1)),(slice(None),slice(1,None)))]:
                fill=mask[source]&~expanded[dest]
                outputs[key][dest][fill,:3]=outputs[key][source][fill,:3]
                expanded[dest]|=fill
            mask=expanded
    current=original.outputs['Color']
    for side in outputs:
        img=bpy.data.images.get(f'{slug}-blink-{side}') or bpy.data.images.new(f'{slug}-blink-{side}',width=w,height=h,alpha=True)
        if tuple(img.size)!=(w,h):img.scale(w,h)
        img.pixels.foreach_set(outputs[side].ravel());img.scale(2048,2048);img.filepath_raw=str(STAGE/f'{slug}-blink-{side}.png');img.file_format='PNG';img.save();img.pack()
        if not build_nodes or side in ['closed','half']:continue
        tex=nodes.new('ShaderNodeTexImage');tex.image=img
        mix=nodes.new('ShaderNodeMixRGB');mix.blend_type='MIX';links.new(current,mix.inputs[1]);links.new(tex.outputs['Color'],mix.inputs[2])
        driver=mix.inputs[0].driver_add('default_value').driver;driver.expression='min(1,max(0,(blink-0.25)/0.5))'
        var=driver.variables.new();var.name='blink';var.targets[0].id_type='KEY';var.targets[0].id=mesh.data.shape_keys;var.targets[0].data_path=f'key_blocks["eyeBlink_{side}"].value'
        current=mix.outputs[0]
    if not build_nodes:return original.outputs['Color'],bsdf.inputs['Base Color'].links[0].from_socket
    # R texture must only override its own patch, otherwise it resets the L texture.
    both=nodes.new('ShaderNodeTexImage');both.image=bpy.data.images[f'{slug}-blink-closed']
    mix=nodes.new('ShaderNodeMixRGB');links.new(current,mix.inputs[1]);links.new(both.outputs['Color'],mix.inputs[2])
    driver=mix.inputs[0].driver_add('default_value').driver;driver.expression='min(l,r)'
    for name,side in [('l','L'),('r','R')]:
        var=driver.variables.new();var.name=name;var.targets[0].id_type='KEY';var.targets[0].id=mesh.data.shape_keys;var.targets[0].data_path=f'key_blocks["eyeBlink_{side}"].value'
    links.new(mix.outputs[0],bsdf.inputs['Base Color'])
    return original.outputs['Color'],mix.outputs[0]

if __name__=='__main__':
    selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS
    for slug in selected:build(slug)
