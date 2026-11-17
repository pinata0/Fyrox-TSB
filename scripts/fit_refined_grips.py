"""Fit shorter fingers and the opposing thumb around the handle clearance."""
import bpy,json,math,sys
import numpy as np
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];DEST=ROOT/'artifacts/hand-mesh-polish/characters'
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx
slugs=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else 'vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
def unit(v):return v/np.linalg.norm(v)
for slug in slugs:
    bpy.ops.wm.open_mainfile(filepath=str(DEST/f'{slug}.blend'));rig=bpy.data.objects['TSB_Rig']
    body=bpy.data.objects[f'{slug}_SkinnedMesh']
    for shape in body.data.shape_keys.key_blocks:shape.value=0
    profiles=json.loads((DEST/f'{slug}-hands.json').read_text())
    for p in profiles:
        f=np.array(p['forward']);a=np.array(p['across']);n=np.array(p['dorsal'])
        directions=[unit(f*.45+a*.89-n*.02),unit(f*.20-a*.05-n*.98),unit(f*.60-a*.10-n*.79),unit(f*.85-a*.05-n*.525)]
        for c in p['controls']:
            i=int(c['name'].split('.')[0][-1])-1
            if c['name'].startswith('Pinky'):c['angle']=[.30,.85,.75][i]
            elif c['name'].startswith(('Index','Ring')):c['angle']=[.50,.95,1.05][i]
            elif c['name'].startswith('Thumb'):
                axis=unit(np.cross(directions[i],directions[i+1]));c['axis']=list(rig.data.bones[p['hand']].matrix_local.to_3x3().inverted()@Vector(axis))
                c['angle']=math.acos(float(np.clip(np.dot(directions[i],directions[i+1]),-1,1)))
    (DEST/f'{slug}-hands.json').write_text(json.dumps(profiles,indent=2))
    record=json.loads((DEST/f'{slug}-rig.json').read_text());record['hands']=profiles
    (DEST/f'{slug}-rig.json').write_text(json.dumps(record,indent=2));bpy.context.scene['hand_rig']=json.dumps(profiles)
    bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(DEST/f'{slug}.blend'))
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and any(m.type=='ARMATURE' and m.object==rig for m in o.modifiers)]
    export_fbx(body,rig,DEST/f'{slug}.fbx',extras)
    print('REFINED_GRIP_FIT_OK',slug,flush=True)
