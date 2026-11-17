"""Inspect connected facial surfaces and their front projection."""
import bpy,json,sys
import numpy as np
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'artifacts/cheek-isolation'
selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else ['hilde','ui','chloe']
for slug in selected:
    bpy.ops.wm.open_mainfile(filepath=str(ROOT/f'assets/characters/{slug}.blend'))
    body=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig'];rig.data.pose_position='REST'
    for shape in body.data.shape_keys.key_blocks:shape.value=0
    face=json.loads((ROOT/f'assets/characters/{slug}-rig.json').read_text())['face']
    points=np.array([v.co[:] for v in body.data.vertices]);adj=[[] for _ in points]
    used=set()
    for p in body.data.polygons:used.update(p.vertices)
    for e in body.data.edges:
        a,b=e.vertices;adj[a].append(b);adj[b].append(a)
    comps=[];lookup=np.full(len(points),-1)
    while used:
        first=used.pop();stack=[first];component=[first]
        while stack:
            for j in adj[stack.pop()]:
                if j in used:used.remove(j);stack.append(j);component.append(j)
        cid=len(comps);lookup[component]=cid;p=points[component]
        in_face=(np.abs(p[:,0]-face['center_x'])<.4)&(p[:,2]>face['mouth_y']-.12)&(p[:,2]<face['eye_y']+.20)&(p[:,1]<-face['front']+.15)
        comps.append(dict(id=cid,count=len(component),face_vertices=int(sum(in_face)),low=p.min(0).tolist(),high=p.max(0).tolist()))
    (OUT/f'{slug}-components.json').write_text(json.dumps(sorted(comps,key=lambda c:c['face_vertices'],reverse=True),indent=2))
    scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=16;scene.render.resolution_x=800;scene.render.resolution_y=800;scene.render.resolution_percentage=100;scene.view_settings.view_transform='AgX'
    for obj in list(bpy.data.objects):
        if obj.type=='MESH':obj.hide_render=obj!=body
        if obj.type in ['CAMERA','LIGHT']:bpy.data.objects.remove(obj,do_unlink=True)
    center=Vector((face['center_x'],-face['front'],(face['eye_y']+face['mouth_y'])/2))
    if scene.world is None:scene.world=bpy.data.worlds.new('Face review world')
    scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[1].default_value=.6
    for d,power in [((1,-2,2),80),((-1,-1,1),55)]:
        bpy.ops.object.light_add(type='AREA',location=center+Vector(d));o=bpy.context.object;o.data.energy=power;o.data.size=2;o.rotation_euler=(center-o.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=center+Vector((0,-3,0)));cam=bpy.context.object;cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=.85;scene.camera=cam
    scene.render.filepath=str(OUT/f'{slug}-face-source.png');bpy.ops.render.render(write_still=True)
    for cid in range(len(comps)):
        mat=bpy.data.materials.new(f'component_{cid}');mat.diffuse_color=tuple(np.random.default_rng(cid+5).uniform(.12,.9,3))+(1,)
        mat.use_nodes=True;mat.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=mat.diffuse_color
        body.data.materials.append(mat)
    for poly in body.data.polygons:poly.material_index=int(lookup[poly.vertices[0]])+1
    scene.render.filepath=str(OUT/f'{slug}-face-components.png');bpy.ops.render.render(write_still=True)
    print('FACE_COMPONENTS',slug,[(c['id'],c['count'],c['face_vertices']) for c in sorted(comps,key=lambda c:c['face_vertices'],reverse=True)[:12]],flush=True)
