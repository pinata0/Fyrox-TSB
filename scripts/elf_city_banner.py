"""Shared banner geometry for the city builder and the saved-scene patcher."""
import math
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[1]
TEXTURE = ROOT / 'assets/maps/elf-city/elena-banner.png'
PREFIX = 'Elena banner'
LEGACY = ('Civic crest panel', 'Municipal gear insignia', 'Gear tooth',
          'Left elf ear', 'Right elf ear', 'Civic core')


def is_replaced(obj):
    return obj.name.startswith((PREFIX,) + LEGACY)


def solid_material(name, color):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.use_nodes = True
    mat['srgb'] = color
    linear = lambda v: v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4
    rgba = tuple(linear(v / 255) for v in color) + (1,)
    mat.diffuse_color = rgba
    bsdf = mat.node_tree.nodes.get('Principled BSDF')
    bsdf.inputs['Base Color'].default_value = rgba
    bsdf.inputs['Roughness'].default_value = .85
    return mat


def add_banner():
    assert TEXTURE.is_file(), 'Run scripts/fetch_elena_banner.py first'
    for obj in list(bpy.context.scene.objects):
        if is_replaced(obj):
            bpy.data.objects.remove(obj, do_unlink=True)
    collection = bpy.data.collections.get('Elena city hall banner')
    if collection is None:
        collection = bpy.data.collections.new('Elena city hall banner')
        bpy.context.scene.collection.children.link(collection)
    canvas = solid_material('ElenaBannerCanvas', [90, 95, 122])
    metal = solid_material('ElenaBannerMount', [187, 175, 137])
    art = bpy.data.materials.get('ElenaBannerArtwork') or bpy.data.materials.new('ElenaBannerArtwork')
    art.use_nodes = True
    art.diffuse_color = (1, 1, 1, 1)
    bsdf = art.node_tree.nodes.get('Principled BSDF')
    bsdf.inputs['Roughness'].default_value = .9
    image = bpy.data.images.load(str(TEXTURE), check_existing=True)
    image.filepath = '//elena-banner.png'
    # Pack the source in the editable .blend; the FBX uses the adjacent PNG.
    image.pack()
    tex = art.node_tree.nodes.get('Elena original illustration')
    if tex is None:
        tex = art.node_tree.nodes.new('ShaderNodeTexImage')
        tex.name = 'Elena original illustration'
    tex.image = image
    art.node_tree.links.new(tex.outputs['Color'], bsdf.inputs['Base Color'])
    art.node_tree.links.new(tex.outputs['Alpha'], bsdf.inputs['Alpha'])
    art.node_tree.links.new(tex.outputs['Color'], bsdf.inputs['Emission Color'])
    bsdf.inputs['Emission Strength'].default_value = .22
    art.surface_render_method = 'DITHERED'
    art.use_backface_culling = True

    def cloth(name, width, height, offset, material):
        nx, ny = 12, 20
        verts, faces = [], []
        for j in range(ny + 1):
            v = j / ny
            for i in range(nx + 1):
                u = i / nx
                game_z = 18 + (u - .5) * width
                game_y = 8.65 + (v - .5) * height
                # A small fold, fixed at both hanging bars; artwork follows it.
                fold = .018 * math.sin((game_z - 16.225) / 3.55 * math.pi * 4) * math.sin((game_y - 5.85) / 5.6 * math.pi)
                verts.append((55.28 + fold - offset, -game_z, game_y))
        for j in range(ny):
            for i in range(nx):
                a = j * (nx + 1) + i
                faces.extend([(a, a + 1, a + nx + 2), (a, a + nx + 2, a + nx + 1)])
        mesh = bpy.data.meshes.new(name)
        mesh.from_pydata(verts, [], faces)
        mesh.materials.append(material)
        uv = mesh.uv_layers.new(name='Banner UV')
        for loop in mesh.loops:
            index = loop.vertex_index
            uv.data[loop.index].uv = ((index % (nx + 1)) / nx, (index // (nx + 1)) / ny)
        obj = bpy.data.objects.new(name, mesh)
        collection.objects.link(obj)
        obj['source'] = 'elena-banner-source.json'
        return obj

    cloth(PREFIX + ' canvas', 3.55, 5.6, 0, canvas)
    height = 5.22
    cloth(PREFIX + ' full body', height * image.size[0] / image.size[1], height, .012, art)
    for y in (5.82, 11.48):
        bpy.ops.mesh.primitive_cube_add(size=1, location=(55.29, -18, y))
        obj = bpy.context.object
        obj.name = PREFIX + ' hanging bar'
        obj.scale = (.11, 3.75, .09)
        for old in list(obj.users_collection):
            old.objects.unlink(obj)
        collection.objects.link(obj)
        obj.data.materials.append(metal)
        # Blender's primitive cube includes UVs.
    return [obj for obj in collection.objects if obj.name.startswith(PREFIX)]
