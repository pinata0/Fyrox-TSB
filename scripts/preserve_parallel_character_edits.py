"""Merge the independently delivered Id mount into the refined facial assets."""
import bpy,json,sys,hashlib
import numpy as np
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx,resolve_character_textures
ASSETS=ROOT/'assets/characters';candidate=ROOT/'artifacts/id-mount-size/candidate/id.blend'
if candidate.exists():
    bpy.ops.wm.open_mainfile(filepath=str(candidate))
    geometry={name:np.array([v.co[:] for v in bpy.data.objects[name].data.vertices],dtype=np.float32) for name in ['Crescent','Nata']}
    bones={name:(bpy.data.objects['TSB_Rig'].data.bones[name].head_local.copy(),bpy.data.objects['TSB_Rig'].data.bones[name].tail_local.copy()) for name in ['CrescentRoot','NataRoot']}
    metadata=bpy.context.scene.get('id_mount_resize')
    bpy.ops.wm.open_mainfile(filepath=str(ASSETS/'id.blend'))
    body=bpy.data.objects['id_SkinnedMesh'];rig=bpy.data.objects['TSB_Rig']
    before=hashlib.sha256(np.array([v.co[:] for v in body.data.vertices],dtype=np.float32).tobytes()).hexdigest()
    for name,points in geometry.items():
        mesh=bpy.data.objects[name].data;assert len(mesh.vertices)==len(points)
        mesh.vertices.foreach_set('co',points.ravel());mesh.update()
    bpy.ops.object.select_all(action='DESELECT');rig.select_set(True);bpy.context.view_layer.objects.active=rig
    bpy.ops.object.mode_set(mode='EDIT')
    for name,(head,tail) in bones.items():rig.data.edit_bones[name].head=head;rig.data.edit_bones[name].tail=tail
    bpy.ops.object.mode_set(mode='OBJECT')
    if metadata:bpy.context.scene['id_mount_resize']=metadata
    assert before==hashlib.sha256(np.array([v.co[:] for v in body.data.vertices],dtype=np.float32).tobytes()).hexdigest()
    extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and o.parent==rig]
    resolve_character_textures([body,*extras])
    for image in bpy.data.images:
        filename=Path(bpy.path.abspath(image.filepath)).name
        if (ASSETS/filename).is_file():image.filepath='//'+filename
    bpy.context.scene.frame_set(0);bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'id.blend'),relative_remap=False)
    export_fbx(body,rig,ASSETS/'id.fbx',extras)
    print('ID_MOUNT_AND_REFINED_FACE_PRESERVED',flush=True)
