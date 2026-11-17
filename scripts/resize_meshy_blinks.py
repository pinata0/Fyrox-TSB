"""Keep closed-eye atlas filtering and resolution consistent with the base albedo."""
import bpy
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
DEST=ROOT/'assets/characters'
for path in sorted(DEST.glob('*-blink-*.png')):
    image=bpy.data.images.load(str(path))
    if tuple(image.size)!=(2048,2048):
        image.scale(2048,2048)
        image.filepath_raw=str(path);image.file_format='PNG';image.save()
    bpy.data.images.remove(image)
for path in sorted(DEST.glob('*.blend')):
    if '-weapon' in path.stem or path.stem.startswith('id-'):continue
    bpy.ops.wm.open_mainfile(filepath=str(path))
    for image in bpy.data.images:
        if '-blink-' not in image.name:continue
        filename=image.filepath.replace('\\','/').split('/')[-1]
        image.unpack(method='REMOVE') if image.packed_file else None
        image.filepath='//'+filename
        image.reload();image.pack()
    bpy.context.preferences.filepaths.save_version=0
    bpy.ops.wm.save_as_mainfile(filepath=str(path))
    print('MESHY_BLINK_RESIZED',path.stem,flush=True)
