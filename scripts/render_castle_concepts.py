"""Render saved editable castle cameras without rebuilding/exporting the scene.
blender --background --python scripts/render_castle_concepts.py -- bedroom study throne
Omit the trailing names to render all nine concept rooms.
"""
import bpy
import sys
from pathlib import Path

root=Path(__file__).resolve().parents[1]
names=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
if not names:names=['bedroom','study','street','cafe','square','atrium','corridor','guardian-hall','throne']
bpy.ops.wm.open_mainfile(filepath=str(root/'assets/maps/fairy-kingdom-castle/fairy-kingdom-castle.blend'))
scene=bpy.context.scene
scene.cycles.samples=16
scene.render.resolution_percentage=80
for name in names:
    scene.camera=bpy.data.objects['Concept '+name]
    scene.render.filepath=str(root/'artifacts/fairy-kingdom-castle'/('concept-'+name+'.png'))
    bpy.ops.render.render(write_still=True)
print('CASTLE_CONCEPT_RENDER_OK',','.join(names),flush=True)
