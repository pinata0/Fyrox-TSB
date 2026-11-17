"""Bake gentle painted shading and contact occlusion onto the rebuilt arms."""
import bpy,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
DEST=ROOT/'artifacts/hand-mesh-polish/characters'
slugs=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else 'vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
for slug in slugs:
    bpy.ops.wm.open_mainfile(filepath=str(DEST/f'{slug}.blend'))
    scene=bpy.context.scene;rig=bpy.data.objects['TSB_Rig'];body=bpy.data.objects[f'{slug}_SkinnedMesh']
    meshes=[o for o in bpy.data.objects if o.type=='MESH' and '_Refined' in o.name]
    materials={m for obj in meshes for m in obj.data.materials}
    if materials and all(not m.name.startswith('Color_') for m in materials):
        print('REFINED_ARM_TEXTURE_ALREADY_BAKED',slug,flush=True);continue
    rig.data.pose_position='REST';scene.render.engine='CYCLES';scene.cycles.samples=16
    bpy.ops.object.select_all(action='DESELECT')
    hidden={o.name:o.hide_render for o in bpy.data.objects if o.type=='MESH'}
    for o in bpy.data.objects:
        if o.type=='MESH':o.hide_render=o not in meshes
    for obj in meshes:obj.select_set(True)
    bpy.context.view_layer.objects.active=meshes[0]
    bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(angle_limit=1.15,island_margin=.015);bpy.ops.object.mode_set(mode='OBJECT')
    image=bpy.data.images.new(f'{slug}-refined-arms',width=1024,height=1024,alpha=False)
    image.filepath_raw=str(DEST/f'{slug}-refined-arms.png');image.file_format='PNG'
    saved=[]
    for m in materials:
        nodes=m.node_tree.nodes;links=m.node_tree.links;bsdf=nodes['Principled BSDF'];out=nodes.get('Material Output')
        base=tuple(bsdf.inputs['Base Color'].default_value)
        geometry=nodes.new('ShaderNodeNewGeometry')
        dot=nodes.new('ShaderNodeVectorMath');dot.operation='DOT_PRODUCT';dot.inputs[1].default_value=(.25,-.55,.80)
        links.new(geometry.outputs['Normal'],dot.inputs[0])
        tone=nodes.new('ShaderNodeMath');tone.operation='MULTIPLY_ADD';tone.inputs[1].default_value=.22;tone.inputs[2].default_value=.78
        links.new(dot.outputs['Value'],tone.inputs[0])
        ao=nodes.new('ShaderNodeAmbientOcclusion');ao.inputs['Color'].default_value=base;ao.inputs['Distance'].default_value=.025;ao.only_local=True
        tint=nodes.new('ShaderNodeMixRGB');tint.blend_type='MULTIPLY';tint.inputs[0].default_value=1
        links.new(ao.outputs['Color'],tint.inputs[1]);links.new(tone.outputs[0],tint.inputs[2])
        emit=nodes.new('ShaderNodeEmission');links.new(tint.outputs[0],emit.inputs[0]);links.new(emit.outputs[0],out.inputs['Surface'])
        target=nodes.new('ShaderNodeTexImage');target.image=image;nodes.active=target
        saved.append((m,bsdf,out,target,[geometry,dot,tone,ao,tint,emit]))
    scene.render.bake.margin=12;scene.render.bake.use_clear=True
    bpy.ops.object.bake(type='EMIT')
    image.save()
    for m,bsdf,out,target,temporary in saved:
        m.node_tree.links.new(bsdf.outputs[0],out.inputs['Surface'])
        for node in temporary:m.node_tree.nodes.remove(node)
        m.node_tree.links.new(target.outputs['Color'],bsdf.inputs['Base Color'])
        bsdf.inputs['Base Color'].default_value=(1,1,1,1);m.diffuse_color=(1,1,1,1)
        m.name=f'{slug}_refined_'+m.name.split('_',2)[-1];m['meshy_base_texture']=image.name
    for obj in bpy.data.objects:
        if obj.name in hidden:obj.hide_render=hidden[obj.name]
    rig.data.pose_position='POSE';scene.frame_set(0)
    bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(DEST/f'{slug}.blend'))
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and any(m.type=='ARMATURE' and m.object==rig for m in o.modifiers)]
    export_fbx(body,rig,DEST/f'{slug}.fbx',extras)
    print('REFINED_ARM_TEXTURE_OK',slug,flush=True)
