"""Repair invalid character image paths without changing geometry or motions."""
import bpy, json, shutil, sys, hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from export_fyrox_fbx import export_fbx, resolve_character_textures
ASSETS=ROOT/'assets/characters'
WORK=ROOT/'artifacts/id-mount-size'
BACKUP=WORK/'texture-repair-before'
BACKUP.mkdir(parents=True,exist_ok=True)
slugs=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else 'vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()
reports=[]
for slug in slugs:
    bpy.ops.wm.open_mainfile(filepath=str(ASSETS/f'{slug}.blend'))
    meshes=[o for o in bpy.context.scene.objects if o.type=='MESH']
    invalid=[n.image for o in meshes for m in o.data.materials if m and m.use_nodes for n in m.node_tree.nodes
             if n.type=='TEX_IMAGE' and n.image and not Path(bpy.path.abspath(n.image.filepath)).is_file()]
    if not invalid:
        continue
    for suffix in ['.blend','.fbx']:
        dest=BACKUP/f'{slug}{suffix}'
        if not dest.exists():shutil.copy2(ASSETS/dest.name,dest)
    resolve_character_textures(meshes)
    bpy.context.preferences.filepaths.save_version=0
    bpy.data.use_autopack=False
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/f'{slug}.blend'),relative_remap=False)
    body=bpy.data.objects[f'{slug}_SkinnedMesh']
    export_fbx(body,bpy.data.objects['TSB_Rig'],ASSETS/f'{slug}.fbx',[m for m in meshes if m!=body])
    reports.append(dict(slug=slug,repaired_images=len(set(invalid))))
    print('CHARACTER_TEXTURE_LINKS_REPAIRED',slug,flush=True)
records=json.loads((ASSETS/'manifest.json').read_text(encoding='utf-8'))
for row in records:
    if (BACKUP / (row['slug'] + '.fbx')).is_file():
        for key in ['blend','model']:
            row['sha256'][key]=hashlib.sha256((ROOT/row[key]).read_bytes()).hexdigest()
(ASSETS/'manifest.json').write_text(json.dumps(records,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(WORK/'texture-repair.json').write_text(json.dumps(reports,indent=2))
