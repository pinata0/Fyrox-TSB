"""Bake separate cheek skin controls from the production mesh and UV atlas.

Coordinates stay in the FBX source basis, so UV seam duplicates share weights.
The surface is never cut: protected features remain joined with zero weights.
"""
import bpy, json, sys, heapq
import numpy as np
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets/characters'
OUT = ROOT / 'artifacts/cheek-isolation'
SLUGS = 'vivi naia erpin id aya picora chloe renua mayo bella ui butter komi hilde shion'.split()

def smooth(t):
    t = np.clip(t, 0, 1)
    return t*t*t*(t*(t*6-15)+10)

def distances(points, adjacency, seeds, allowed=None):
    distance = np.full(len(points), np.inf)
    queue = []
    for i in seeds: distance[i] = 0; heapq.heappush(queue, (0., int(i)))
    while queue:
        d, i = heapq.heappop(queue)
        if d > distance[i]: continue
        for j, length in adjacency[i]:
            if allowed is not None and not allowed[j]: continue
            nd = d + length
            if nd < distance[j]: distance[j] = nd; heapq.heappush(queue, (nd, j))
    return distance

def build(slug, render=False, publish=False):
    bpy.ops.wm.open_mainfile(filepath=str(ASSETS / f'{slug}.blend'))
    body = bpy.data.objects[f'{slug}_SkinnedMesh']; mesh = body.data
    bpy.data.objects['TSB_Rig'].data.pose_position = 'REST'
    for shape in mesh.shape_keys.key_blocks: shape.value = 0
    face = json.loads((ASSETS / f'{slug}-rig.json').read_text())['face']
    p = np.array([v.co[:] for v in mesh.vertices]); x,y,z = p.T
    cx,ex,ez,mz,eh,front = [face[k] for k in ['center_x','eye_x','eye_y','mouth_y','eye_half_height','front']]
    adjacency = [[] for _ in p]
    for edge in mesh.edges:
        a,b = edge.vertices; length = float(np.linalg.norm(p[a]-p[b]))
        adjacency[a].append((int(b),length)); adjacency[b].append((int(a),length))
    image = bpy.data.images.load(str(ASSETS / f'{slug}-meshy-basecolor.png'), check_existing=False)
    w,h = image.size; pixels = np.array(image.pixels[:], dtype=np.float32).reshape(h,w,4)
    color = np.zeros((len(p),3)); count = np.zeros(len(p))
    for loop,uv in zip(mesh.loops,mesh.uv_layers.active.data):
        u,v = uv.uv; i = loop.vertex_index
        color[i] += pixels[min(h-1,max(0,int(v*h))), min(w-1,max(0,int(u*w))), :3]; count[i] += 1
    color /= np.maximum(count[:,None],1)
    r,g,b = color.T
    # Warm skin and blush, excluding grey frames, green hair and dark outlines.
    skin = (r>.55)&(r>=g*.98)&(r>=b*.98)&(g>r*.43)&(b>g*.90)
    eye = (((np.abs(x-cx)-ex)/(ex*.76))**2 + ((z-ez)/(eh*1.06))**2) < 1.0
    eye |= (np.abs(np.abs(x-cx)-ex)<.08)&(np.abs(z-ez)<eh*.85)
    from face_regions import eye_triangles
    mesh.calc_loop_triangles()
    eye |= eye_triangles(p,np.array([t.vertices[:] for t in mesh.loop_triangles]),face)
    mouth = ((x-cx)/(.10 if slug=='ui' else .080))**2 + ((z-mz)/.036)**2 < 1
    roi = (np.abs(x-cx)<.40)&(z>mz-.14)&(z<ez+.05)&(y<-front+.22)
    # Fill small painted marks (Chloe's heart, cheek spirals); keep large hair regions out.
    todo = set(np.flatnonzero(roi & ~skin & ~eye & ~mouth))
    while todo:
        first=todo.pop(); part=[first]; stack=[first]
        while stack:
            for j,_ in adjacency[stack.pop()]:
                if j in todo: todo.remove(j); part.append(j); stack.append(j)
        bounds = np.ptp(p[part],axis=0)
        touches_feature=any(eye[j] or mouth[j] or not roi[j] for i in part for j,_ in adjacency[i])
        if len(part)<1200 and bounds[0]<.085 and bounds[2]<.065 and not touches_feature: skin[part]=True
    # Hilde's white glasses are fused to the head. Protect their complete rim,
    # including bridges and antialiased edge vertices; never fill those islands.
    glasses = np.zeros(len(p),dtype=bool)
    accessory = np.zeros(len(p),dtype=bool)
    if slug=='hilde':
        glasses = roi & (y < -front+.035)
        for _ in range(1):
            expanded = glasses.copy()
            for i in np.flatnonzero(glasses):
                for j,_ in adjacency[i]:
                    if abs(r[j]-g[j])<.10 and abs(g[j]-b[j])<.11: expanded[j]=True
            glasses=expanded
        skin &= ~glasses
    if slug=='ui':
        # Two pink hair clips overlap the right cheek in projection, in front of skin.
        accessory=(x>cx+.21)&(z>mz+.035)&(y<-.17)
    protected = ~roi | ~skin | eye | mouth | glasses | accessory
    clearance = distances(p,adjacency,np.flatnonzero(protected))
    guard = smooth(clearance/.027)
    centers=[]; weights=[]; gradients=[]; stats=[]
    for side in [-1,1]:
        target=np.array([cx+side*(ex+.085), -front, mz+.035])
        blush=roi & ~protected & (side*(x-cx)>ex+.035)&(side*(x-cx)<ex+.19)&(z>mz-.02)&(z<mz+.085)&(g<r*.85)&(b>g*.96)&(r>.7)
        if sum(blush)>4: target=np.mean(p[blush],axis=0)
        target[0]=cx+side*np.clip(side*(target[0]-cx),ex+.055,ex+.12)
        target[2]=np.clip(target[2],mz+.027,mz+.060)
        candidates = (~protected)&(side*(x-cx)>ex+.025)&(side*(x-cx)<ex+.16)&(z>mz+.012)&(z<min(mz+.080,ez-.025))&(clearance>.0265)
        indices=np.flatnonzero(candidates)
        if not len(indices):
            sample=roi&(side*(x-cx)>ex+.025)&(side*(x-cx)<ex+.12)&(z>mz+.015)&(z<min(mz+.085,ez-.015))
            print('APEX_DEBUG',slug,[(int(i),p[i].round(4).tolist(),color[i].round(3).tolist(),bool(skin[i]),bool(eye[i]),round(clearance[i],3)) for i in np.flatnonzero(sample)],flush=True)
            raise RuntimeError(f'{slug}: no unprotected cheek apex')
        score=.2*y[indices] + 8*(x[indices]-target[0])**2 + 8*(z[indices]-target[2])**2
        apex=int(indices[np.argmin(score)]); center=p[apex].copy(); centers.append(center.tolist())
        d=p-center
        q=(d[:,0]/.235)**2+(d[:,2]/.155)**2+(d[:,1]/.20)**2
        # A rounded core and C2 falloff avoid the flat, sharply folded old tip.
        weight=(1-smooth(np.sqrt(q)/1.12))*guard
        weight[protected]=0
        # A skin-only geodesic prevents crossing detached/occluding hair sheets.
        geodesic=distances(p,adjacency,[apex],allowed=~protected)
        weight *= 1-smooth((geodesic-.19)/.13)
        weight[~np.isfinite(geodesic)]=0
        weight[np.abs(weight)<1e-6]=0
        # UV-independent tangent gradients for inverse-transpose normal transport.
        gradient=np.zeros_like(p)
        for i in np.flatnonzero(weight>0):
            neighbors=[j for j,_ in adjacency[i]]
            if neighbors:
                delta=p[neighbors]-p[i]
                gradient[i]=np.linalg.lstsq(np.vstack([delta,np.eye(3)*.005]),np.r_[weight[neighbors]-weight[i],np.zeros(3)],rcond=None)[0]
        centers[-1]=[round(float(v),7) for v in center]
        weights.append(weight);gradients.append(gradient)
        stats.append(dict(apex_vertex=apex,apex_weight=float(weight[apex]),moving_vertices=int(sum(weight>.001)),core_vertices=int(sum(weight>.8)),core_height=float(np.ptp(z[weight>.8])),core_width=float(np.ptp(x[weight>.8]))))
        assert weight[apex]>.9 and sum(weight>.2)>25,(slug,side,stats[-1])
    # Include every vertex, so runtime validation also checks untouched hair/mouth/glasses.
    entries=[]
    for i,point in enumerate(p):
        flags=(1 if eye[i] else 0)|(2 if mouth[i] else 0)|(4 if glasses[i] or accessory[i] else 0)|(8 if protected[i] else 0)
        entries.append([*[float(v) for v in point],*[round(float(a[i]),6) for a in weights],*[round(float(v),5) for gradient in gradients for v in gradient[i]],flags])
    result=dict(version=1,slug=slug,centers=centers,hit_radius=.14,vertices=entries,stats=stats,protected=dict(eyes=int(sum(eye&roi)),mouth=int(sum(mouth&roi)),glasses=int(sum(glasses))))
    (ASSETS/f'{slug}-cheeks.json').write_text(json.dumps(result,separators=(',',':')))
    print('CHEEK_MASK_OK',slug,json.dumps({k:result[k] for k in ['centers','stats','protected']}),flush=True)
    if publish:
        sys.path.insert(0,str(ROOT/'scripts'))
        from export_fyrox_fbx import export_fbx, resolve_character_textures
        for vertex in mesh.vertices:
            groups=sorted([(g.group,g.weight) for g in vertex.groups],key=lambda a:a[1],reverse=True)
            if len(groups)>4 or abs(sum(w for _,w in groups)-1)>1e-5:
                for group,_ in groups:body.vertex_groups[group].remove([vertex.index])
                total=sum(w for _,w in groups[:4])
                for group,weight in groups[:4]:body.vertex_groups[group].add([vertex.index],weight/total,'REPLACE')
        for i,side in enumerate(['L','R']):
            sign=-1 if i==0 else 1
            for channel,delta in [('cheekStretch',np.array([sign*.12,-.066,0])),('cheekPuff',np.array([0,-.03,0])),('cheekSquash',np.array([-sign*.025,.015,0]))]:
                key=mesh.shape_keys.key_blocks[f'{channel}_{side}'];key.value=0
                key.data.foreach_set('co',(p+weights[i][:,None]*delta).astype(np.float32).ravel())
            attribute=mesh.attributes.get(f'cheek_weight_{side}') or mesh.attributes.new(f'cheek_weight_{side}','FLOAT','POINT')
            attribute.data.foreach_set('value',weights[i].astype(np.float32))
        rig=bpy.data.objects['TSB_Rig'];rig.data.pose_position='POSE';bpy.context.scene.frame_set(0)
        extras=[o for o in bpy.data.objects if o.type=='MESH' and o!=body and o.parent==rig]
        resolve_character_textures([body,*extras])
        for image in bpy.data.images:
            filename=Path(bpy.path.abspath(image.filepath)).name
            if image.source=='FILE' and (ASSETS/filename).is_file():image.filepath='//'+filename
        bpy.context.preferences.filepaths.save_version=0
        bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/f'{slug}.blend'))
        export_fbx(body,rig,ASSETS/f'{slug}.fbx',extras)
        rig.data.pose_position='REST'
    if render: render_review(body,face,np.array(weights),centers,slug)
    return {k:result[k] for k in ['slug','centers','stats','protected']}

def render_review(body,face,weights,centers,slug):
    scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=16;scene.render.threads_mode='FIXED';scene.render.threads=4
    scene.render.resolution_x=800;scene.render.resolution_y=800;scene.render.resolution_percentage=100;scene.view_settings.view_transform='AgX'
    for obj in list(bpy.data.objects):
        if obj.type=='MESH':obj.hide_render=obj!=body
        if obj.type in ['CAMERA','LIGHT']:bpy.data.objects.remove(obj,do_unlink=True)
    center=Vector((face['center_x'],-face['front'],(face['eye_y']+face['mouth_y'])/2))
    if scene.world is None:scene.world=bpy.data.worlds.new('Cheek review world')
    scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[1].default_value=.6
    for d,power in [((1,-2,2),80),((-1,-1,1),55)]:
        bpy.ops.object.light_add(type='AREA',location=center+Vector(d));obj=bpy.context.object;obj.data.energy=power;obj.data.size=2;obj.rotation_euler=(center-obj.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add(location=center+Vector((0,-3,0)));cam=bpy.context.object;cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.type='ORTHO';cam.data.ortho_scale=.85;scene.camera=cam
    for tag,delta in [('neutral',np.zeros(3)),('pull',np.array([-.12,-.06875,.035]))]:
        if tag=='pull':
            key=body.shape_key_add(name='Cheek review',from_mix=False);key.value=1
            for i,v in enumerate(key.data): v.co+=Vector(delta*weights[0,i])
        scene.render.filepath=str(OUT/f'{slug}-{tag}.png');bpy.ops.render.render(write_still=True)
    if slug=='chloe':
        for key in body.data.shape_keys.key_blocks:key.value=0
        for side in ['L','R']:
            body.data.shape_keys.key_blocks[f'eyeBlink_{side}'].value=1
            body.data.shape_keys.key_blocks[f'browDown_{side}'].value=.35
        scene.render.filepath=str(OUT/f'{slug}-closed.png');bpy.ops.render.render(write_still=True)

if __name__=='__main__':
    args=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
    selected=[s for s in args if not s.startswith('--')] or SLUGS
    report=[build(slug,'--render' in args,'--publish-rig' in args) for slug in selected]
    (OUT/'mask-report.json').write_text(json.dumps(report,indent=2))
