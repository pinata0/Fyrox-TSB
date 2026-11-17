"""Verify saved Spawn animation variation, idle seam, timing, and manifest hashes."""
import hashlib
import json
from pathlib import Path
import bpy

ROOT = Path(__file__).resolve().parents[1]
records = json.loads((ROOT / 'assets/characters/manifest.json').read_text(encoding='utf-8'))
source = json.loads((ROOT / 'assets/characters/spawn/motions.json').read_text(encoding='utf-8'))
reports = []
for item, sampled in zip(records, source):
    slug = item['slug']
    assert slug == sampled['slug']
    assert item['clips']['Spawn'] == [456, 528]
    assert item['spawn']['duration'] == sampled['duration']
    for key in ['blend', 'model']:
        assert hashlib.sha256((ROOT / item[key]).read_bytes()).hexdigest() == item['sha256'][key]
    bpy.ops.wm.open_mainfile(filepath=str(ROOT / item['blend']))
    rig = bpy.data.objects['TSB_Rig']

    def pose(frame):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        return [v for p in rig.pose.bones for row in p.matrix for v in row]

    idle = pose(0)
    end = pose(528)
    seam = max(abs(a-b) for a, b in zip(idle, end))
    assert seam < .0001, (slug, 'idle seam', seam)
    start = pose(456)
    delta = max(abs(a-b) for frame in [474, 492, 510] for a,b in zip(start, pose(frame)))
    assert delta > .001, (slug, 'static spawn')
    reports.append({'slug': slug, 'duration': sampled['duration'], 'idle_seam': seam, 'motion_delta': delta})
    print('SPAWN_VERIFY_OK', slug, 'seam', seam, 'motion', delta, flush=True)
assert len(reports) == 15
(ROOT / 'artifacts/spawn-motions/saved-asset-verification.json').write_text(
    json.dumps(reports, indent=2), encoding='utf-8')
print('SPAWN_VERIFY_COMPLETE', len(reports), flush=True)
