"""Independent saved Blend + FBX validation for the four-digit hand revision."""
import json
import sys
from pathlib import Path
import bmesh
import bpy
import numpy as np

sys.path.insert(0,str(Path(__file__).resolve().parent))
from cartoon_hands import OUT, STAGE, SLUGS, DIGITS, fingerprint, neutral, pose


def verify(slug):
    bpy.ops.wm.open_mainfile(filepath=str(STAGE/f'{slug}.blend'))
    rig=bpy.data.objects['TSB_Rig'];record=json.loads((STAGE/f'{slug}-rig.json').read_text())
    profiles=json.loads((STAGE/f'{slug}-hands.json').read_text())
    for name,digest in record['cartoon_hands']['untouched_meshes'].items():
        assert fingerprint(bpy.data.objects[name])==digest,(slug,name,'unrelated mesh changed')
    assert len(profiles)==2 and {p['side'] for p in profiles}=={'L','R'}
    assert len(rig.data.bones)==record['bones']
    neutral(rig);reports=[]
    for p in profiles:
        expected={f'{digit}{segment}.{p["side"]}' for digit in DIGITS for segment in (1,2)}
        assert len(p['controls'])==8 and {c['name'] for c in p['controls']}==expected
        assert {b.name for b in rig.data.bones if b.name.endswith('.'+p['side']) and b.name.startswith((*DIGITS,'Ring'))}==expected
        obj=bpy.data.objects[f'{slug}_RefinedHand.{p["side"]}']
        bm=bmesh.new();bm.from_mesh(obj.data)
        assert all(e.is_manifold for e in bm.edges),(slug,p['side'],'open/nonmanifold mesh')
        assert all(face.calc_area()>1e-10 for face in bm.faces),(slug,p['side'],'degenerate face')
        pending=set(bm.verts);components=0
        while pending:
            components+=1;stack=[pending.pop()]
            while stack:
                for edge in stack.pop().link_edges:
                    for v in edge.verts:
                        if v in pending:pending.remove(v);stack.append(v)
        bm.free();assert components==1,(slug,p['side'],'detached fingers')
        assert obj.data.uv_layers.active
        assert all(0<len(v.groups)<=4 and abs(sum(g.weight for g in v.groups)-1)<1e-4 for v in obj.data.vertices)
        rest=np.array([v.co[:] for v in obj.data.vertices]);scale=p['radius']/.013
        w,f,a,n=(np.array(p[key]) for key in ('wrist','forward','across','dorsal'))
        # Above the finger webs the actual surface must split into three fingers.
        selected=set(int(i) for i in np.flatnonzero(((rest-w)@f)>.082*scale))
        neighbors=[[] for _ in rest]
        for edge in obj.data.edges:
            x,y=edge.vertices;neighbors[x].append(y);neighbors[y].append(x)
        islands=[]
        while selected:
            stack=[selected.pop()];part=[]
            while stack:
                i=stack.pop();part.append(i)
                for j in neighbors[i]:
                    if j in selected:selected.remove(j);stack.append(j)
            islands.append(part)
        assert len(islands)==3,(slug,p['side'],'finger silhouette',list(map(len,islands)))
        pose(rig,[p]);dg=bpy.context.evaluated_depsgraph_get()
        closed=np.array([v.co[:] for v in obj.evaluated_get(dg).data.vertices]);assert np.isfinite(closed).all()
        delta=closed-np.array(p['contact']);radial=np.sqrt((delta@f)**2+(delta@n)**2)
        digits=[]
        for digit in DIGITS:
            indices={g.index for g in obj.vertex_groups if g.name.startswith(digit)}
            mask=np.array([sum(g.weight for g in v.groups if g.group in indices)>.65 for v in obj.data.vertices])
            assert mask.sum()>30,(slug,p['side'],digit,'no skin weights')
            movement=float(np.quantile(np.linalg.norm(closed-rest,axis=1)[mask],.9))
            assert movement>.008,(slug,p['side'],digit,'no articulation')
            gap=float(np.min(np.abs(radial[mask]-p['radius'])))
            penetration=float(np.quantile(np.maximum(p['radius']-radial[mask],0),.99))
            distal_group=obj.vertex_groups[f'{digit}2.{p["side"]}'].index
            distal=np.array([any(g.group==distal_group and g.weight>.7 for g in v.groups) for v in obj.data.vertices])
            tip_gap=float(np.min(np.abs(radial[distal]-p['radius'])))
            assert gap<.003,(slug,p['side'],digit,'grip gap',gap)
            assert penetration<.002,(slug,p['side'],digit,'grip penetration',penetration)
            assert tip_gap<.003,(slug,p['side'],digit,'distal segment floating',tip_gap)
            digits.append(dict(name=digit,vertices=int(mask.sum()),grip_gap=gap,distal_grip_gap=tip_gap,penetration_p99=penetration,movement=movement))
        edges=np.array([e.vertices[:] for e in obj.data.edges])
        strain=np.linalg.norm(closed[edges[:,0]]-closed[edges[:,1]],axis=1)/np.maximum(np.linalg.norm(rest[edges[:,0]]-rest[edges[:,1]],axis=1),1e-8)
        assert float(np.quantile(strain,.99))<2.5,(slug,p['side'],'skin stretch')
        reports.append(dict(side=p['side'],vertices=len(rest),closed_surface=True,connected=True,finger_tip_islands=3,digits=digits,edge_stretch_p99=float(np.quantile(strain,.99))))
        pose(rig,[p],0)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(STAGE/f'{slug}.fbx'))
    rig=next(o for o in bpy.data.objects if o.type=='ARMATURE')
    body=next(o for o in bpy.data.objects if o.type=='MESH' and o.name.startswith(slug+'_SkinnedMesh'))
    assert len(rig.data.bones)==record['bones'] and len(body.data.shape_keys.key_blocks)==24
    assert any(a.frame_range[1]-a.frame_range[0]>=527 for a in bpy.data.actions),(slug,'lost animation')
    for image in bpy.data.images:
        if image.source=='FILE':assert Path(bpy.path.abspath(image.filepath)).is_file(),image.filepath
    for p in profiles:
        assert all(c['name'] in rig.data.bones for c in p['controls'])
    return dict(slug=slug,passed=True,bones=record['bones'],hands=reports,fbx_roundtrip=True,morphs=23,unrelated_meshes_unchanged=True)


if __name__=='__main__':
    selected=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else SLUGS
    reports=[]
    for slug in selected:
        reports.append(verify(slug))
        (OUT/'verification.json').write_text(json.dumps(dict(passed=len(reports)==len(selected),characters=reports),indent=2))
        print('CARTOON_HANDS_VERIFY_OK',slug,flush=True)
