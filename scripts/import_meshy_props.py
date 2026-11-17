"""Prepare separately editable props, textures and game-scale FBX assets."""
import bpy
import bmesh
import json
import math
import sys
import numpy as np
from pathlib import Path
from mathutils import Vector, Matrix
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from import_meshy_characters import setup_scene, STAGE, OUT

SIZES={'vivi':1.05,'naia':.68,'erpin':1.2,'aya':1.35,'picora':.85,'renua':.60,'mayo':.65,'bella':.40,'ui':.72,'butter':.48,'komi':.52,'hilde':.65,'shion':.95,'id':1.3}

def run(slug,kind):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    folder=ROOT/f'assets/character-sources/meshy/{slug}/{kind}'
    bpy.ops.import_scene.fbx(filepath=str(folder/f'{slug}-{kind}.fbx'))
    objects=[o for o in bpy.context.scene.objects if o.type=='MESH']
    assert objects
    bpy.context.view_layer.update()
    bpy.ops.object.select_all(action='DESELECT')
    for o in objects:
        world=o.matrix_world.copy();o.parent=None;o.matrix_world=world;o.select_set(True)
    bpy.context.view_layer.objects.active=objects[0];bpy.ops.object.join();mesh=bpy.context.object
    bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
    bm=bmesh.new();bm.from_mesh(mesh.data);bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.000001);bm.to_mesh(mesh.data);bm.free()
    mod=mesh.modifiers.new('Runtime detail budget','DECIMATE');mod.ratio=min(1,12000/max(1,len(mesh.data.polygons)));bpy.ops.object.modifier_apply(modifier=mod.name)
    points=np.array([v.co[:] for v in mesh.data.vertices]);lo=points.min(0);hi=points.max(0);size=hi-lo
    center=(lo+hi)*.5
    points-=center
    # The source front faces -Y. Long guns/blowpipe align with engine forward (-Y here).
    rotation=Matrix.Identity(3)
    if slug in ['picora','vivi']:
        rotation=Vector((-1,0,0)).rotation_difference(Vector((0,0,1))).to_matrix()
    if slug in ['hilde','shion','mayo']:
        axis=int(np.argmax(size));direction=Vector((0,-1,0));source=Vector([1 if a==axis else 0 for a in range(3)])
        rotation=source.rotation_difference(direction).to_matrix()
    points=points@np.array(rotation).T
    factor=(.52 if kind=='robot' else SIZES[slug])/max(np.ptp(points,axis=0))
    points*=factor
    # Grip-aligned props use the existing weapon contact coordinates.
    if slug in ['erpin','picora','ui']:points[:,2]+=.18
    elif slug=='aya':points[:,2]+=.48
    elif slug=='vivi':points[:,2]+=.43
    elif slug in ['naia','mayo','hilde','shion']:points[:,1]-=.16;points[:,2]+=.04
    elif slug=='bella':points[:,2]+=.42
    elif slug=='butter':points[:,2]+=.12
    mesh.data.vertices.foreach_set('co',points.astype(np.float32).ravel());mesh.data.update();mesh.name=f'{slug}_{kind}'
    mesh.data.materials.clear();mat=bpy.data.materials.new(f'{slug}_meshy_{kind}');mat.use_nodes=True;mesh.data.materials.append(mat)
    for polygon in mesh.data.polygons:polygon.material_index=0;polygon.use_smooth=True
    image=bpy.data.images.load(str(folder/f'{slug}-{kind}-basecolor.png'));image.scale(1024,1024)
    image.filepath_raw=str(STAGE/f'{slug}-{kind}-meshy-basecolor.png');image.file_format='PNG';image.save();image.pack()
    bsdf=mat.node_tree.nodes.get('Principled BSDF');bsdf.inputs['Roughness'].default_value=.8
    tex=mat.node_tree.nodes.new('ShaderNodeTexImage');tex.image=image;mat.node_tree.links.new(tex.outputs['Color'],bsdf.inputs['Base Color'])
    for o in list(bpy.data.objects):
        if o!=mesh:bpy.data.objects.remove(o,do_unlink=True)
    if slug=='butter':
        mesh.shape_key_add(name='Basis',from_mix=False).value=0
        k=mesh.shape_key_add(name='bandPull',from_mix=False);k.value=0
        for i,p in enumerate(points):
            strength=math.exp(-(p[0]/.115)**4-((p[2]-.18)/.085)**4)
            k.data[i].co.y+=.32*strength
    # Native empties remain editable anchors in the authoring file.
    for name,p in [('PropRoot',(0,0,0)),('Grip',(0,0,0)),('Muzzle',(0,float(points[:,1].min()),float(np.median(points[:,2]))))]:
        o=bpy.data.objects.new(name,None);bpy.context.collection.objects.link(o);o.location=p
    bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}-{kind}.blend'))
    bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True);bpy.context.view_layer.objects.active=mesh
    bpy.ops.export_scene.fbx(filepath=str(STAGE/f'{slug}-{kind}.fbx'),use_selection=True,object_types={'MESH'},axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
    # Render a centered enlarged source review, without modifying exported geometry.
    points=np.array([v.co[:] for v in mesh.data.vertices]);mid=(points.min(0)+points.max(0))*.5
    mesh.location=Vector((0,0,.93))-Vector(mid);mesh.scale=(1.4/max(np.ptp(points,axis=0)),)*3
    mesh.location=Vector((0,0,.93))-Vector(mid)*mesh.scale.x
    setup_scene(mesh,OUT/f'{slug}-{kind}-prop.png')
    record=dict(slug=slug,kind=kind,vertices=len(mesh.data.vertices),triangles=len(mesh.data.polygons),bounds=[points.min(0).tolist(),points.max(0).tolist()],source_dimensions=size.tolist())
    (STAGE/f'{slug}-{kind}-prop.json').write_text(json.dumps(record,indent=2))
    print('MESHY_PROP_OK',slug,kind,flush=True)

if __name__=='__main__':
    manifest=json.loads((ROOT/'assets/character-sources/meshy/manifest.json').read_text())
    for record in manifest['entries']:
        if record['kind'] in ['weapon','mount','robot']:run(record['slug'],record['kind'])
