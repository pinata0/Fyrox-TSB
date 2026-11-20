"""Fit source remesh coordinates to the current game rest pose (Blender CLI)."""
import json
from pathlib import Path
import numpy as np
from mathutils.kdtree import KDTree

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/lowpoly-import'
inventory=json.loads((OUT/'inventory.json').read_text())
slugs='vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()

def fit(source,target):
    tree=KDTree(len(target))
    for i,p in enumerate(target): tree.insert(p,i)
    tree.balance()
    scale=np.ptp(target[:,2])/np.ptp(source[:,2])
    rotation=np.eye(3)
    offset=(target.min(0)+target.max(0))/2-(source.min(0)+source.max(0))/2*scale
    sample=source[np.linspace(0,len(source)-1,min(2200,len(source))).astype(int)]
    for _ in range(35):
        p=sample@rotation.T*scale+offset
        closest=[tree.find(v) for v in p]
        q=np.array([v[0] for v in closest]);distance=np.array([v[2] for v in closest])
        good=distance<=np.quantile(distance,.72)
        a=sample[good];b=q[good];ac=a.mean(0);bc=b.mean(0)
        u,_,vt=np.linalg.svd((a-ac).T@(b-bc))
        fix=np.eye(3);fix[-1,-1]=np.linalg.det(vt.T@u.T)
        rotation=vt.T@fix@u.T
        scale=np.sum(((a-ac)@rotation.T)*(b-bc))/np.sum((a-ac)**2)
        offset=bc-ac@rotation.T*scale
    distances=np.array([tree.find(p)[2] for p in source@rotation.T*scale+offset])
    return dict(scale=float(scale),rotation=rotation.tolist(),offset=offset.tolist(),
        median=float(np.median(distances)),p90=float(np.quantile(distances,.9)),p99=float(np.quantile(distances,.99)))

if __name__=='__main__':
    candidates={s:np.load(OUT/(s+'.npz'))[s+'_SkinnedMesh'] for s in slugs}
    records=[]
    for row in inventory:
        if not row['file'].startswith('temp/'):continue
        if row['meshes'][0]['triangles']<7000:continue
        source=np.load(OUT/(Path(row['file']).stem+'.npz'))[row['meshes'][0]['name']]
        matches={slug:fit(source,target) for slug,target in candidates.items()}
        ranked=sorted(matches,key=lambda s:matches[s]['median'])
        records.append(dict(file=row['file'],matches=matches,ranked=ranked))
        print(Path(row['file']).stem,[(s,round(matches[s]['median'],4),round(matches[s]['p90'],4)) for s in ranked[:3]],flush=True)
        (OUT/'matches.json').write_text(json.dumps(records,indent=2))
