"""Compare neutral, old closed and repaired closed face paint at the same UVs."""
import bpy
import json
import sys
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
sys.dont_write_bytecode = True
from review_face_paint import sample

OUT = ROOT / 'artifacts/face-feature-fix'
ASSETS = ROOT / 'assets/characters'
tiles = []
for slug in (sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else json.loads((ASSETS/'blush-regions.json').read_text())):
    d = np.load(OUT/f'{slug}-projection.npz')
    colors = [d['base'].copy(), sample(ASSETS/f'{slug}-blink-closed.png', d['uv']), sample(OUT/f'candidate/{slug}-blink-closed.png', d['uv'])]
    np.save(OUT/f'{slug}-repaired-projection.npy',colors[-1])
    for color in colors:
        color[~d['valid']] = [.2, .2, .2, 1]
    tiles.append(colors[-1])
    pixels = np.concatenate(colors, axis=1)
    image = bpy.data.images.new(slug, width=pixels.shape[1], height=pixels.shape[0], alpha=True)
    image.pixels.foreach_set(pixels.ravel())
    image.filepath_raw = str(OUT/f'{slug}-comparison.png')
    image.file_format = 'PNG'
    image.save()
    print('FACE_COMPARISON_OK', slug, flush=True)
if len(tiles) == 15:
    # Blender pixels run bottom-to-top; put alphabetical roster order at top.
    rows = [np.concatenate(tiles[i:i+5], axis=1) for i in range(0, 15, 5)]
    pixels = np.concatenate(rows[::-1], axis=0)
    image = bpy.data.images.new('Roster closed blush', width=pixels.shape[1], height=pixels.shape[0], alpha=True)
    image.pixels.foreach_set(pixels.ravel())
    image.filepath_raw = str(OUT/'all-closed.png')
    image.file_format = 'PNG'
    image.save()
