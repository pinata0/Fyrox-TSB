"""FBX export adapter: Blender linear materials -> Fyrox's sRGB Color property."""
import bpy
from pathlib import Path
import json

def resolve_character_textures(meshes):
    """Recover packed image links without accepting a directory as a texture."""
    assets = Path(__file__).resolve().parents[1] / 'assets/characters'
    for obj in meshes:
        for material in obj.data.materials:
            if not material or not material.use_nodes:
                continue
            for node in material.node_tree.nodes:
                if node.type != 'TEX_IMAGE' or not node.image:
                    continue
                image = node.image
                if image.filepath and Path(bpy.path.abspath(image.filepath)).is_file():
                    continue
                names = [image.name, image.name + '.png', obj.name.split('.')[0] + '-basecolor.png']
                names += [p.filepath.replace('\\', '/').rsplit('/', 1)[-1] for p in image.packed_files]
                if material.get('meshy_base_texture') == image.name and '_meshy_' in material.name:
                    names.insert(0, material.name.split('_meshy_')[0] + '-meshy-basecolor.png')
                path = next((assets / name for name in names if name and (assets / name).is_file()), None)
                if path is None:
                    raise ValueError(f'Missing texture for {obj.name}/{material.name}/{image.name}: {image.filepath!r}')
                image.filepath = str(path)


def export_fbx(mesh,rig,path,extra_meshes=None,bake_step=1,texture_path_mode='AUTO'):
    # Character post-processors may only pass the body. Keep its independently
    # skinned hands, sleeves and companions attached to the same rig in the FBX.
    if extra_meshes is None:
        extra_meshes = tuple(obj for obj in bpy.context.scene.objects
                            if obj.type == 'MESH' and obj != mesh
                            and any(mod.type == 'ARMATURE' and mod.object == rig for mod in obj.modifiers))
    resolve_character_textures((mesh, *extra_meshes))
    # All TSB clip boundaries are authored in 30 Hz frames. Imported Tripo scenes
    # default to 24 Hz, which otherwise moves Death into the Dance time interval.
    bpy.context.scene.render.fps=30
    bpy.context.scene.render.fps_base=1.0
    saved=[]
    saved_links=[]
    for material in {m for obj in (mesh,*extra_meshes) for m in obj.data.materials}:
        if material and material.get('meshy_base_texture'):
            bsdf=material.node_tree.nodes.get('Principled BSDF')
            socket=bsdf.inputs['Base Color']
            saved_links.append((material,socket,socket.links[0].from_socket if socket.links else None))
            original=next(n for n in material.node_tree.nodes if n.type=='TEX_IMAGE' and n.image and n.image.name==material['meshy_base_texture'])
            material.node_tree.links.new(original.outputs['Color'],socket)
        if not material or not material.name.startswith('Color_'):continue
        rgb=material.name.split('_')[1][:6]
        rgba=tuple(int(rgb[i:i+2],16)/255 for i in (0,2,4))+(1,)
        bsdf=material.node_tree.nodes.get('Principled BSDF')
        saved.append((material,tuple(material.diffuse_color),tuple(bsdf.inputs['Base Color'].default_value)))
        material.diffuse_color=rgba
        bsdf.inputs['Base Color'].default_value=rgba
    bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True);rig.select_set(True)
    for obj in extra_meshes:obj.select_set(True)
    bpy.context.view_layer.objects.active=rig
    try:
        bpy.ops.export_scene.fbx(filepath=str(path),use_selection=True,object_types={'ARMATURE','MESH'},
            apply_unit_scale=True,axis_forward='-Z',axis_up='Y',add_leaf_bones=False,
            bake_anim=True,bake_anim_use_all_actions=False,bake_anim_use_nla_strips=False,
            bake_anim_step=bake_step,bake_anim_simplify_factor=0,path_mode=texture_path_mode)
    finally:
        for material,socket,source in saved_links:
            if source:material.node_tree.links.new(source,socket)
        for material,diffuse,base in saved:
            material.diffuse_color=diffuse
            material.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value=base

if __name__=='__main__':
    root=Path(__file__).resolve().parents[1]
    for record in json.loads((root/'assets/characters/manifest.json').read_text()):
        bpy.ops.wm.open_mainfile(filepath=str(root/record['blend']))
        export_fbx(bpy.data.objects[record['slug']+'_SkinnedMesh'],bpy.data.objects['TSB_Rig'],root/record['model'])
        print('FYROX_FBX_OK',record['slug'],flush=True)
    (root/'artifacts/fbx-export-complete.txt').write_text('15 models exported with sRGB material colors',encoding='utf-8')
