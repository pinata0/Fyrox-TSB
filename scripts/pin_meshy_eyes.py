"""Keep painted eye regions fixed under independently authored cheek controls."""
import bpy,json,sys
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from import_meshy_characters import STAGE,SLUGS
from export_fyrox_fbx import export_fbx
for slug in SLUGS:
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    mesh=bpy.data.objects[f'{slug}_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig'];f=json.loads((STAGE/f'{slug}-rig.json').read_text())['face']
    rest=np.array([p.co[:] for p in mesh.data.shape_keys.key_blocks['Basis'].data]);x,y,z=rest.T
    protected=(np.abs(np.abs(x-f['center_x'])-f['eye_x'])<.11)&(np.abs(z-f['eye_y'])<f['eye_half_height']*1.4)
    for k in mesh.data.shape_keys.key_blocks:
        if k.name.startswith('cheek'):
            for i in np.flatnonzero(protected):k.data[int(i)].co=rest[i]
    mesh.data.vertices.foreach_set('co',rest.astype(np.float32).ravel());mesh.data.update()
    bpy.context.scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    export_fbx(mesh,rig,STAGE/f'{slug}.fbx',[o for o in bpy.context.scene.objects if o.type=='MESH' and o!=mesh])
    print('MESHY_EYE_GUARD_OK',slug,flush=True)
