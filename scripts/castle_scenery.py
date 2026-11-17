"""Nine concept-art spaces on the retained Anubis layout. Blender scene assembly."""
from mathutils import Matrix
exec(compile((ROOT/'scripts/castle_concept_assets.py').read_text(encoding='utf-8'),'castle_concept_assets.py','exec'),globals())

def inside_footprint(x,z):
    poly=layout['footprint'];inside=False
    for a,b in zip(poly,poly[1:]+poly[:1]):
        if (a[1]>z)!=(b[1]>z) and x<(b[0]-a[0])*(z-a[1])/(b[1]-a[1])+a[0]:inside=not inside
    return inside

def transform_new(before,x,z,turn=0,y=0,prop=None):
    bpy.context.view_layer.update()
    transform=Matrix.Translation(Vector(loc((x,y,z))))@Matrix.Rotation(turn,4,'Z')
    objects=set(bpy.data.objects)-before
    for o in objects:
        o.matrix_world=transform@o.matrix_world
        if prop:
            o['asset_id']=prop['id'];o['asset_kind']=prop['kind'];o['concept_images']=prop['reference'];o['space']=prop['space']
    return objects

def placed_window(name,p,w,h,turn=0,pointed=False):
    before=set(bpy.data.objects);window(name,0,0,0,w,h,pointed);transform_new(before,p[0],p[2],turn,p[1])

def column(name,p,h,r):
    x,y,z=p
    lathe(name+' turned stone shaft',(x,y,z),[(0,r),(.2,r),(.3,r*.8),(.7,r*.68),(h-.8,r*.55),(h-.6,r*.72),(h-.35,r*.9),(h-.15,r),(h,r)],SLATE,n=20)
    for dy,rr in [(.25,.85),(.7,.72),(h-.7,.74),(h-.32,1.05)]:
        cylinder(name+' carved collar',(x,y+dy,z),r*rr,.12,CREAM,24)
    for j in range(12):
        a=j*math.tau/12
        pipe(name+' flute',[(x+math.cos(a)*r*.67,y+.8,z+math.sin(a)*r*.67),(x+math.cos(a)*r*.56,y+h-.9,z+math.sin(a)*r*.56)],.022,CARVE,5)

def vault(name,x0,x1,z0,z1,spring,rise):
    vs=[];fs=[];n=25
    for layer in [0,.14]:
        for x in [x0,x1]:
            for j in range(n):
                t=j/(n-1);vs.append((x,spring+rise*math.sin(t*math.pi)+layer,z0+(z1-z0)*t))
    for j in range(n-1):
        fs.extend([(j,j+1,j+1+n,j+n),(j+2*n,j+3*n,j+3*n+1,j+2*n+1),
                   (j,j+2*n,j+2*n+1,j+1),(j+n,j+n+1,j+3*n+1,j+3*n)])
    fs.extend([(0,n,3*n,2*n),(n-1,3*n-1,4*n-1,2*n-1)])
    mesh_object(name+' stone vault',vs,fs,SLATE)
    for x in [x0+(x1-x0)*t for t in [0,.25,.5,.75,1]]:
        pts=[(x,spring+rise*math.sin(j*math.pi/24)-.08,z0+(z1-z0)*j/24) for j in range(25)]
        pipe(name+' rib arch',pts,.14,CREAM)

collection('01 Limestone streets and wood interiors')
poly=layout['footprint'];n=len(poly)
vs=[(x,y,z) for y in [-.4,0] for x,z in poly]
fs=[tuple(reversed(range(n))),tuple(range(n,2*n))]+[(i,(i+1)%n,(i+1)%n+n,i+n) for i in range(n)]
mesh_object('Bent castle foundation',vs,fs,GRASS)
batches=[([],[]) for _ in STONE_TILES]
for row in range(92):
    z=-53.5+row
    for col in range(76):
        x=-21.4+col*1.2+(row%2)*.6
        if not all(inside_footprint(x+a,z+b) for a,b in [(-.6,-.5),(.6,.5),(-.6,.5),(.6,-.5)]):continue
        vs,fs=batches[(col+row+random.randrange(3))%4];i=len(vs)
        a=random.uniform(.015,.07);b=random.uniform(.015,.06)
        vs.extend([(x-.6+a,.015,z-.5+b),(x+.6-a,.015,z-.5+b*.5),(x+.6-a*.5,.015,z+.5-b),(x-.6+a,.015,z+.5-b*.5)])
        fs.append((i+3,i+2,i+1,i))
for mat,(vs,fs) in zip(STONE_TILES,batches):mesh_object('Irregular courtyard flagstones',vs,fs,mat)
# Fine-seamed marble replaces exterior paving in the five white stone rooms.
for x0,x1,z0,z1 in [(-1.95,11.95,-49,-41),(10.7,17.3,9.7,18.3),(24.2,53.8,-3.8,4.8),(24.2,37.8,25.2,34.8),(28.2,53.8,5.2,24.8)]:
    cube('Interior marble floor',((x0+x1)/2,.025,(z0+z1)/2),(x1-x0,.02,z1-z0),MARBLE)
    for x in range(math.ceil(x0),math.floor(x1),3):pipe('Marble joint',[(x,.037,z0),(x,.037,z1)],.008,MARBLE_VEIN,4)
    for z in range(math.ceil(z0),math.floor(z1),3):pipe('Marble joint',[(x0,.037,z),(x1,.037,z)],.008,MARBLE_VEIN,4)
    for j in range(int((x1-x0)*(z1-z0)/4)):
        x=random.uniform(x0+.4,x1-.4);z=random.uniform(z0+.4,z1-.4)
        pipe('Soft stone vein',[(x-.25,.037,z-.15),(x-.11,.038,z-.08),(x+.08,.037,z+.14),(x+.27,.037,z+.21)],.005,MARBLE_VEIN,4)
for x0,x1,z0,z1,y in [(-17.8,-6.2,-28.8,-9.2,.03),(10.2,21.8,-27.3,-12.7,.62),(5.5,10,-25.5,-14.5,.62)]:
    for i in range(int((x1-x0)/.45)):
        x=x0+i*.45+.22
        cube('Oak floor plank',(x,y,(z0+z1)/2),(.43,.025,z1-z0),OAK if i%3 else WOOD)
        for j in range(2):pipe('Fine wood grain',[(x-.1+j*.2,y+.018,z0+.2),(x-.08+j*.2,y+.018,(z0+z1)/2),(x-.12+j*.2,y+.018,z1-.2)],.006,WOOD,4)
for x,_,z in layout['points']:
    cylinder('Objective limestone inlay',(x,.025,z),3.7,.035,SLATE,64)
    pipe('Objective inlay ring',[(x+3.5*math.cos(a),.05,z+3.5*math.sin(a)) for a in [i*math.tau/64 for i in range(65)]],.035,GOLD)

collection('02 Shared structure and classical columns')
for i,b in enumerate(layout['blocks']):
    p,s=b['center'],b['size'];name=b['name']
    if 'column' in name.lower() or name=='A crescent support':
        column(name,(p[0],p[1]-s[1]/2,p[2]),s[1],min(s[0],s[2])*.48);continue
    if not b.get('render',True):continue
    mat=OAK if name.startswith('Cafe') and ('step' in name or 'terrace' in name or 'interior' in name) else MATS.get(tuple(b['color']),SLATE)
    if name.startswith(('Bedroom','Attack garden lodge')):mat=BEDROOM_STONE
    elif name.endswith('roof') and name.startswith(('Approach west','B north','Royal defense','Forward garden')):mat=SLATE
    if 'arch' in b:
        vs,fs=lintel_geometry(b)
        o=mesh_object('%03d %s'%(i,name),vs,fs,mat)
        cube('Wall carved stringcourse',(p[0],p[1]+s[1]/2-.2,p[2]),(s[0]+.08,.15,s[2]+.08),CREAM,.025)
    else:
        o=cube('%03d %s'%(i,name),p,s,mat,.035)
    o['collision_index']=i
    if 'arch' not in b and s[1]>2 and min(s[0],s[2])<.6:
        for y in [.22,p[1]+s[1]/2-.2]:cube('Wall carved stringcourse',(p[0],y,p[2]),(s[0]+.08,.15,s[2]+.08),CREAM,.025)
    if 'escarpment' in name:cube('Upper garden lawn',(p[0],14.05,p[2]),(s[0],.1,s[2]),GRASS)

collection('03 Green tiled timber town and cafe')
for r in layout['roofs']:
    x,z,y,w,d=r['x'],r['z'],r['y'],r['w'],r['d'];style=r.get('style','palace')
    if r['name'] in ['Approach west house','Approach east gatehouse']:
        count=3 if style=='timber' else 2
        for i in range(count):
            before=set(bpy.data.objects)
            gabled_roof(r['name'],0,y,0,d/count-.15,w,style=='cafe')
            transform_new(before,x,z+(i-(count-1)/2)*d/count,-math.pi/2)
        continue
    if style=='timber':gabled_roof(r['name'],x,y,z,w,d);continue
    if style=='atrium':
        # Open central skylight for the indoor tree; no solid dome above it.
        for xx in [x-w/2,x+w/2]:cube('Atrium skylight curb',(xx,y,z),(.3,.45,d),SLATE)
        for zz in [z-d/2,z+d/2]:cube('Atrium skylight curb',(x,y,zz),(w,.45,.3),SLATE)
        continue
    # Palace wings use narrow, arched roof pavilions with carved cornices.
    if w>d*1.5:
        count=max(2,round(w/7))
        for i in range(count):dome(r['name'],x+(i-(count-1)/2)*w/count,y,z,min(d,w/count)*.47,2.3)
    else:dome(r['name'],x,y,z,w*.47,2.8,d/w)

# Timber framing follows solid facade portions; doors are never crossed at head height.
for x,zs,lo,hi,h in [(-5.74,[-28.8,-24,-20,-13,-9.2],-29,-9,6.2),(9.74,[-27.3,-23,-17,-12.7],-27.5,-12.5,5.3)]:
    for z in zs:cube('Exposed timber upright',(x,h/2,z),(.2,h,.22),WOOD,.025)
    for y in [3.65,h-.25]:cube('Timber facade crossbeam',(x,y,(lo+hi)/2),(.22,.22,hi-lo),WOOD,.025)
    spans=[(lo,-18),(-14,hi)] if x<0 else [(lo,hi)]
    for a,b in spans:cube('Timber facade footbeam',(x,.35,(a+b)/2),(.22,.22,b-a),WOOD,.025)
    for z in zs[:-1]:pipe('Timber diagonal brace',[(x-.02,3.8,z),(x-.02,h-.4,z+1.3)],.075,WOOD)
for x,z,turn in [(-5.7,-25,-math.pi/2),(-5.7,-11.3,-math.pi/2),(9.7,-25,math.pi/2),(9.7,-15,math.pi/2)]:
    placed_window('Town arched window',(x,1.3,z),2.1,2.8,turn)
# Cafe porch rails, menu sign and striped awning leave the stairs clear.
for z in [-25.5,-14.5]:
    pipe('Curved terrace handrail',[(5.5,.75,z),(6.3,1.5,z),(9.7,1.6,z)],.1,OAK)
    for x in [6.5,7.5,8.5,9.5]:cube('Terrace baluster',(x,1.05,z),(.07,.9,.07),OAK)
for z0,z1 in [(-25.5,-22.3),(-17.7,-14.5)]:
    pipe('Terrace edge rail',[(5.55,1.5,z0),(5.55,1.5,z1)],.07,OAK)
for i in range(12):
    z=-26.5+i*1.08
    mesh_object('Cafe striped canopy',[(5.7,3.75,z),(10,4.4,z),(10,4.4,z+1.04),(5.7,3.75,z+1.04)],[(0,1,2,3)],LINEN if i%2 else MINT)
    cube('Canopy scalloped hem',(5.7,3.66,z+.52),(.06,.22,1.04),LINEN if i%2 else MINT,.045)
before=set(bpy.data.objects)
sphere('Cafe hanging oval sign',(0,0,0),(1.3,.55,.14),OAK,3)
sphere('Cafe sign inset',(0,0,-.14),(1.1,.43,.025),LINEN,3)
cup(0,-.12,-.18);transform_new(before,9.5,-20,math.pi/2,4.8)
for j in range(11):
    z=-26+j*1.2;y=3.6-.35*math.sin(j*math.pi/10)
    if j:pipe('Cafe festoon cable',[(5.55,3.6-.35*math.sin((j-1)*math.pi/10),z-1.2),(5.55,y,z)],.014,WOOD,5)
    sphere('Cafe festoon bulb',(5.55,y-.1,z),(.08,.12,.08),LIGHT,2)

collection('04 Furnished concept spaces')
asset_report=[]
for prop in layout['props']:
    collection('Object / '+prop['id'])
    before=set(bpy.data.objects);make_prop(prop)
    p=prop['position'];objects=transform_new(before,p[0],p[2],prop['turn'],p[1],prop)
    asset_report.append(dict(**prop,mesh_objects=sum(o.type=='MESH' for o in objects)))
(OUT/'object-catalog.json').write_text(json.dumps(asset_report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')

collection('05 Study woodwork and village festival')
for x in [-17.72,-6.28]:
    lo,hi=(-18.6,-9.4) if x<-10 else (-13.8,-9.4)
    cube('Study olive wainscot',(x,1.65,(lo+hi)/2),(.035,3.3,hi-lo),GREEN)
    for y in [.2,3.3]:cube('Study ivory dado rail',(x,y,(lo+hi)/2),(.08,.13,hi-lo),CREAM)
    for z in [-18.5,-15.5,-12.5,-9.5]:
        if lo<=z<=hi:cube('Study painted pilaster',(x,2.5,z),(.13,5,.15),CREAM)
placed_window('Study sun window',(-6.34,1.25,-11.5),2.4,3.8,math.pi/2)
for z in [-37,-31,-23]:
    pts=[(-5+t*27,5.6-.9*math.sin(t*math.pi),z) for t in [i/20 for i in range(21)]]
    pipe('Festival bunting cord',pts,.018,WOOD,5)
    for i in range(17):
        t=(i+.5)/18;x=-5+t*27;y=5.6-.9*math.sin(t*math.pi)
        mesh_object('Colorful festival pennant',[(x-.32,y,z),(x+.32,y,z),(x,y-.75,z)],[(0,1,2)],[ROSE,TEAL,GOLD,MINT,LINEN][i%5])
for x,z,y in [(18,-23,9),(-15,-25,9.8)]:
    cube('Town chimney',(x,y,z),(.9,2,.9),CREAM,.06)
    cube('Chimney cap',(x,y+1.1,z),(1.1,.2,1.1),SLATE,.04)

collection('06 Light-filled tree atrium')
for x in [10.78,17.22]:
    for z in [10,18]:column('Atrium engaged column',(x,0,z),10.7,.3)
    for z in [12.8,16]:placed_window('Atrium clerestory',(x,5.3,z),2.1,4.7,-math.pi/2 if x<14 else math.pi/2,True)
for z in [9.78,18.22]:placed_window('Atrium high arch',(14,5,z),4.7,5.5,math.pi if z<14 else 0,True)
for z in [9.8,18.2]:
    for x in [11,14,17]:
        ivy('Atrium hanging ivy',x,9.8,z,2.4)
    pipe('Atrium pale garland',[(10.8,2.5,z),(12,2.1,z),(14,2.5,z),(16,2.1,z),(17.2,2.5,z)],.055,LINEN)

collection('07 Vaulted white corridor and guardian hall')
vault('South cloister',24.3,53.7,-3.7,4.7,5.4,1.35)
for x in [26,32,38,44,50,53]:
    column('Cloister engaged column',(x,0,-3.58),5.4,.24)
    for y in [1.2,3.8]:cube('Cloister ashlar seam',(x,y,-3.76),(4.8,.015,.01),CARVE)
for x in [30,35,44,51]:placed_window('Cloister high window',(x,3.6,4.72),2.2,2.4,0)
for x in [28.5,33.5]:placed_window('Guardian hall lancet',(x,1.2,34.73),3.2,6.2,0,True)
placed_window('Guardian hall ceremonial window',(37.73,1.3,27.5),3.5,6.1,math.pi/2,True)
for x in [24.7,31,37.4]:
    for z in [25.4,34.5]:column('Guardian hall pilaster',(x,0,z),7.9,.32)
    ivy('Guardian hall ivy',x,7.6,34.55,2.4)
sphere('Hall moon emblem',(37.56,6.1,27.5),(.035,.45,.45),LIGHT,3)

collection('08 Royal audience hall and red throne')
vault('Royal audience hall',28.3,53.7,5.3,24.7,9,2.55)
for x in [31.5,45.5,51]:
    for z,turn in [(5.28,math.pi),(24.72,0)]:placed_window('Royal hall lancet',(x,6.6,z),3.0,4.5,turn,True)
placed_window('Great throne window',(53.75,2.7,15),7.6,8.7,math.pi/2,True)
for z in [6,11,19,24]:
    column('Throne engaged pillar',(53.35,0,z),11.2,.37)
    before=set(bpy.data.objects);ivy('Throne ivy',0,10.7,0,3.4);transform_new(before,53.12,z,math.pi/2)
for i in range(4):
    x=49.9+i*.6;h=.3*(i+1)
    cube('Marble throne stair',(x,h/2,15),(.6,h,4.2),SLATE,.025)
    cube('Red stair runner',(x,h+.012,15),(.59,.018,2.55),RED)
    cube('Runner down riser',(x-.302,h-.15,15),(.014,.29,2.55),RED)
cube('Marble throne landing',(52.6,.6,15),(1.2,1.2,4.2),SLATE,.035)
cube('Throne landing carpet',(52.6,1.215,15),(1.18,.02,2.55),RED)
cube('Audience red runner',(48.8,.028,15),(1.6,.02,2.55),RED)

collection('09 Castle gates sanctuaries and skyline')
for gate in layout['gates']:
    before=set(bpy.data.objects);w=gate['opening'];h=gate['height']
    for side in [-1,1]:
        cylinder('Gate turret',(side*(w/2+1.2),h+.75,0),1.2,1.5,SLATE)
        dome('Mint gate crown',side*(w/2+1.2),h+1.5,0,1.35,2.1)
    crown('Gate royal crest',0,h-.8,-.26,1)
    if 'arch' in gate:
        pipe('Gate moulding',moulding_points(w,gate['arch']),.1,CREAM)
    else:
        arch('Gate moulding',0,0,-.26,w,3.8,CREAM,.1)
    transform_new(before,gate['x'],gate['z'],math.pi/2 if gate['axis']=='x' else 0)
for x,z in {(p[0],p[2]) for p in layout['spawns'].values()}:
    for dx in [-6.5,6.5]:cube('Sanctuary floor side',(x+dx,.04,z),(.06,.025,6),GOLD)
    for dz in [-3,3]:cube('Sanctuary floor edge',(x,.04,z+dz),(13,.025,.06),GOLD)
for x,z,h in [(-25,-50,9),(25,-50,9),(-25,-8,10),(25,-8,12),(-25,33,11),(22,41,12),(69,39,14),(69,-7,12)]:tower('Perimeter fairy tower',x,z,h,2.2)
before=set(bpy.data.objects)
cube('Fairy palace drum',(0,7,0),(25,14,12),SLATE,.2)
for x in [-10,-5,0,5,10]:
    h=17 if x==0 else 13
    cylinder('Palace tall turret',(x,h/2,-4),2.3 if x else 4,h,SLATE)
    dome('Palace emerald crown',x,h,-4,2.5 if x else 4.3,4 if x else 5.5)
    for dx in [-.8,.8]:window('Distant palace lancet',x+dx,h*.45,-6.25,.8,3.5,True)
beam('World tree trunk',(0,0,17),(1.5,31,19),4.5,OAK)
for i in range(26):
    a=i*2.39996;r=math.sqrt(i/25)*24
    if i%4==0:beam('World tree bough',(1,19,18),(math.cos(a)*r,31,19+math.sin(a)*r*.5),1.5,OAK)
    sphere('Distant world tree canopy',(math.cos(a)*r,33+(1-r/30)*7,19+math.sin(a)*r*.55),(8,5,7),LEAVES[i%4],2)
transform_new(before,77,15,math.pi/2)
for x,z,y in [(30,-12,14),(40,-20,14),(56,-13,14),(34,-36,14),(55,-40,14),(-25,-30,0),(-25,13,0),(12,42,0)]:tree('Beyond wall garden',x,z,y,2.4)
