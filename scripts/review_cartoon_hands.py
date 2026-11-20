"""Render actual saved hand geometry, and measure its two-joint grip."""
import json
import sys
from pathlib import Path
import bpy
import numpy as np
from mathutils import Matrix, Quaternion, Vector

sys.path.insert(0,str(Path(__file__).resolve().parent))
from cartoon_hands import OUT, STAGE, BACK, SLUGS, DIGITS, neutral, pose, unit


def fit(rig,p,obj):
    points=np.array([v.co[:] for v in obj.data.vertices])
    weights={group.name:np.array([next((g.weight for g in v.groups if g.group==group.index),0) for v in obj.data.vertices]) for group in obj.vertex_groups}
    f,n,contact=(np.array(p[key]) for key in ('forward','dorsal','contact'))
    reports=[]
    for digit in DIGITS:
        controls=[c for c in p['controls'] if c['name'].startswith(digit)]
        names=[c['name'] for c in controls]
        base,mid=[np.array(rig.data.bones[name].head_local) for name in names]
        axes=[np.array(rig.data.bones[p['hand']].matrix_local.to_3x3()@Vector(c['axis'])) for c in controls]
        mask=(weights[names[0]]+weights[names[1]])>.65
        xyz=points[mask];w1=weights[names[0]][mask,None];w2=weights[names[1]][mask,None]
        candidates=[]
        angle_sets=(np.linspace(.12,.85,24),np.linspace(.60,1.75,28)) if digit!='Thumb' else (np.linspace(.75,1.8,28),np.linspace(.05,1.0,24))
        for angle1 in angle_sets[0]:
            r1=np.array(Quaternion(Vector(axes[0]),float(angle1)).to_matrix());t1=base-r1@base
            one=xyz@r1.T+t1;qmid=r1@mid+t1
            for angle2 in angle_sets[1]:
                r2=np.array(Quaternion(Vector(axes[1]),float(angle2)).to_matrix())
                two=(one-qmid)@r2.T+qmid
                moved=xyz*(1-w1-w2)+one*w1+two*w2
                delta=moved-contact;radial=np.sqrt((delta@f)**2+(delta@n)**2)
                penetration=float(np.quantile(np.maximum(p['radius']-radial,0),.99))
                gap=float(np.min(np.abs(radial-p['radius'])))
                # Reward several contact vertices; avoid a floating thumb or over-curl.
                distal=w2[:,0]>.7
                near=float(np.quantile(np.abs(radial[distal]-p['radius']),.06))
                score=penetration*30+gap*2+near+abs(angle1+angle2-(1.65 if digit!='Thumb' else 1.6))*.00015
                candidates.append((score,float(angle1),float(angle2),gap,penetration))
        best=min(candidates)
        controls[0]['angle']=best[1];controls[1]['angle']=best[2]
        reports.append(dict(digit=digit,angles=best[1:3],gap=best[3],penetration_p99=best[4]))
    return reports


def render(slug, folder, label, closed=False):
    bpy.ops.wm.open_mainfile(filepath=str(folder/f'{slug}.blend'))
    rig=bpy.data.objects['TSB_Rig'];profiles=json.loads((folder/f'{slug}-hands.json').read_text());p=profiles[0]
    scene=bpy.context.scene;neutral(rig)
    if closed:pose(rig,profiles)
    hand=bpy.data.objects[f'{slug}_RefinedHand.L']
    for obj in list(scene.objects):
        if obj.type in {'LIGHT','CAMERA'}:bpy.data.objects.remove(obj,do_unlink=True)
        elif obj.type=='MESH':obj.hide_render=obj!=hand
    w,f,a,n=(Vector(p[key]) for key in ('wrist','forward','across','dorsal'))
    if closed:
        bpy.ops.mesh.primitive_cylinder_add(vertices=64,radius=p['radius'],depth=.12*(p['radius']/.013),location=Vector(p['contact']))
        handle=bpy.context.object;handle.name='Review grip cylinder'
        handle.rotation_euler=a.to_track_quat('Z','Y').to_euler()
        mat=bpy.data.materials.new('Review grip blue');mat.use_nodes=True
        mat.diffuse_color=(.08,.30,.42,1)
        mat.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=mat.diffuse_color
        mat.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value=.5
        handle.data.materials.append(mat)
        for poly in handle.data.polygons:poly.use_smooth=True
    center=w+f*.050
    scene.render.engine='CYCLES';scene.cycles.samples=24
    scene.render.resolution_x=760;scene.render.resolution_y=760;scene.render.resolution_percentage=100
    scene.view_settings.view_transform='AgX'
    scene.world=bpy.data.worlds.new('Hand study');scene.world.use_nodes=True
    scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.14,.17,.21,1)
    scene.world.node_tree.nodes['Background'].inputs[1].default_value=.6
    for offset,energy,size in [(n*.35-f*.1-a*.25,3,.35),(-n*.30+f*.20+a*.30,2,.3)]:
        bpy.ops.object.light_add(type='AREA',location=center+offset);light=bpy.context.object
        light.data.energy=energy;light.data.shape='DISK';light.data.size=size
        light.rotation_euler=(center-light.location).to_track_quat('-Z','Y').to_euler()
    for view,offset in [('palm',-n*.6+a*.10+f*.05),('back',n*.6+a*.13+f*.06)]:
        bpy.ops.object.camera_add(location=center+offset);camera=bpy.context.object
        direction=(center-camera.location).normalized()
        right=direction.cross(f).normalized();up=right.cross(direction).normalized()
        camera.rotation_euler=Matrix((right,up,-direction)).transposed().to_euler()
        camera.data.type='ORTHO';camera.data.ortho_scale=.26;scene.camera=camera
        scene.render.filepath=str(OUT/f'{slug}-{label}-{view}.png');bpy.ops.render.render(write_still=True)
        bpy.data.objects.remove(camera,do_unlink=True)


if __name__=='__main__':
    args=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else ['erpin']
    for slug in args:
        render(slug,BACK,'before')
        render(slug,STAGE,'open')
        render(slug,STAGE,'grip',True)
