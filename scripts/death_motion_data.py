"""Sample the researched 2D Die curves for retargeting to the existing 3D rigs."""
import json
import math
from pathlib import Path
DATA = json.loads((Path(__file__).resolve().parents[1] / 'assets/characters/death/motions.json').read_text(encoding='utf-8'))

def sample(index, phase):
    frames = DATA[index]['samples']
    at = max(0, min(1, phase)) * (len(frames)-1)
    a = int(at); b = min(a+1, len(frames)-1); t = at-a
    rotations = {name: -(angle+(frames[b]['rotations'][name]-angle)*t) for name,angle in frames[a]['rotations'].items()}
    root = [frames[a]['root'][j]+(frames[b]['root'][j]-frames[a]['root'][j])*t for j in range(2)]
    # 2D exit/fling translations are compressed to the 3D body footprint, preserving their timing.
    location = (0, -1.1*math.tanh(root[0]), max(-.6, min(1.25, root[1])))
    return rotations, location
