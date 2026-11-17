"""Render four poses of each verified dance from the shipped Blender files."""
import json, sys, math
from pathlib import Path
import bpy
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/dance-research/poses'; OUT.mkdir(parents=True,exist_ok=True)
data=json.loads((ROOT/'artifacts/dance-research/sampled-dances.json').read_text(encoding='utf-8'))
args=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
animated='--animation' in args
args=[a for a in args if a!='--animation']
for record in data:
    slug=record['slug']
    if args and slug not in args:continue
    bpy.ops.wm.open_mainfile(filepath=str(ROOT/f'assets/characters/{slug}.blend'))
    scene=bpy.context.scene
    camera=scene.camera
    camera.location=(0,-5,1.12)
    camera.rotation_euler=(Vector((0,0,1.12))-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.type='ORTHO';camera.data.ortho_scale=3.2 if slug=='aya' else 2.7
    scene.render.engine='CYCLES';scene.cycles.samples=8;scene.cycles.use_denoising=True
    scene.render.resolution_x=384;scene.render.resolution_y=384;scene.render.resolution_percentage=100
    scene.render.image_settings.file_format='PNG';scene.render.film_transparent=False
    count=round(record['duration']*30) if animated else 4
    frames=[285+120*i/count for i in range(count)] if animated else [285,300,315,330]
    folder=OUT/slug if animated else OUT
    folder.mkdir(exist_ok=True)
    for phase,frame in enumerate(frames):
        scene.frame_set(math.floor(frame),subframe=frame-math.floor(frame))
        scene.render.filepath=str(folder/f'{phase:03}.png') if animated else str(folder/f'{slug}-{phase}.png')
        bpy.ops.render.render(write_still=True)
    print('DANCE_RENDER_OK',slug,flush=True)
