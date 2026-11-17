"""Make delivered Blender texture links relative to each authoring file."""
import bpy
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
STAGE=ROOT/'artifacts/meshy-import/characters'
for path in sorted(STAGE.glob('*.blend')):
    bpy.ops.wm.open_mainfile(filepath=str(path))
    for image in bpy.data.images:
        filename=Path(image.filepath).name
        if filename and (STAGE/filename).is_file():image.filepath='//'+filename
    bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(path))
    print('MESHY_RELATIVE_PATHS_OK',path.name,flush=True)
