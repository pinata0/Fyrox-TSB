"""Mesh-built object kit extracted from the nine supplied concept paintings.
Executed in the Blender builder namespace. All meshes remain editable in blend.
"""
WOOD=material('Walnut timber',[99,68,47]); OAK=material('Honey oak',[180,131,78])
CREAM=material('Painted cream wood',[235,222,181]); LINEN=material('Warm linen',[253,244,213])
MINT=material('Mint quilt',[132,201,139]); RED=material('Royal vermilion velvet',[183,55,45])
PINK=material('Rose quilt',[210,151,144]); GREEN=material('Study olive walls',[104,134,73])
INK=material('Chalkboard teal',[48,103,92]); GLASS=material('Daylit opal windows',[214,242,238])
SLATE=material('Cool white marble',[211,220,225]); CARVE=material('Carved stone shadow',[124,146,160])
CLAY=material('Terracotta',[170,102,61]); EARTH=material('Rich planter earth',[85,74,43])
BOOKS=[material('Book cloth '+str(i),c) for i,c in enumerate([[44,107,90],[77,135,115],[124,96,73],[169,119,91],[85,101,130]])]
ROOF_TILES=[material('Emerald shingle '+str(i),c) for i,c in enumerate([[54,127,109],[69,149,121],[95,167,123],[44,107,105]])]
RED_TILES=[material('Terracotta shingle '+str(i),c) for i,c in enumerate([[192,84,52],[214,108,63],[228,134,76]])]
LEAVES=[material('Living leaf '+str(i),c) for i,c in enumerate([[58,119,48],[88,155,52],[132,184,64],[177,208,91]])]
STONE_TILES=[material('Street limestone '+str(i),c) for i,c in enumerate([[207,198,176],[222,214,192],[198,191,175],[229,221,197]])]
BEDROOM_STONE=material('Bedroom blue limestone',[137,165,191])
MARBLE=material('Polished white marble',[225,232,236])
MARBLE_VEIN=material('Subtle marble veins',[196,210,217])
MARBLE.node_tree.nodes.get('Principled BSDF').inputs['Roughness'].default_value=.28

def mesh_object(name,verts,faces,mat):
    mesh=bpy.data.meshes.new(name);mesh.from_pydata([loc(v) for v in verts],[],faces);mesh.update()
    o=bpy.data.objects.new(name,mesh);COL.objects.link(o);mesh.materials.append(mat);return o

def pipe(name,pts,r,mat,sides=8):
    pts=[p for i,p in enumerate(pts) if i==0 or sum((p[k]-pts[i-1][k])**2 for k in range(3))>1e-12]
    vs=[];fs=[];world=[Vector(loc(p)) for p in pts];previous_u=None
    for i,p in enumerate(world):
        d=(world[min(i+1,len(world)-1)]-world[max(0,i-1)]).normalized()
        # Transport the frame instead of switching axes at a vertical tangent.
        # A frame flip used to twist arch rings into a black pinched seam.
        if previous_u is None:
            guide=min([Vector((1,0,0)),Vector((0,1,0)),Vector((0,0,1))],key=lambda a:abs(d.dot(a)))
            u=d.cross(guide).normalized()
        else:
            u=(previous_u-d*previous_u.dot(d)).normalized()
            if u.length<.5:
                guide=min([Vector((1,0,0)),Vector((0,1,0)),Vector((0,0,1))],key=lambda a:abs(d.dot(a)))
                u=d.cross(guide).normalized()
        previous_u=u.copy();v=d.cross(u).normalized()
        for j in range(sides):vs.append(tuple(p+r*(u*math.cos(j*math.tau/sides)+v*math.sin(j*math.tau/sides))))
    for i in range(len(pts)-1):
        for j in range(sides):a=i*sides+j;b=i*sides+(j+1)%sides;fs.append((a,b,b+sides,a+sides))
    fs.extend([tuple(reversed(range(sides))),tuple((len(pts)-1)*sides+j for j in range(sides))])
    mesh=bpy.data.meshes.new(name);mesh.from_pydata(vs,[],fs);mesh.update()
    o=bpy.data.objects.new(name,mesh);COL.objects.link(o);mesh.materials.append(mat)
    for p in mesh.polygons:p.use_smooth=True
    return o

def arch_points(w,h,pointed=False):
    spring=h-w/2 if not pointed else h*.55
    pts=[(-w/2,0),(-w/2,spring)]
    for i in range(1,25):
        t=i/24
        if pointed:
            x=(t-.5)*w;y=spring+(h-spring)*(1-abs(2*t-1)**1.5)
        else:x=-math.cos(t*math.pi)*w/2;y=spring+math.sin(t*math.pi)*w/2
        pts.append((x,y))
    pts.append((w/2,0));return pts

def arch(name,x,y,z,w,h,mat=CREAM,thickness=.1,pointed=False):
    return pipe(name,[(x+a,y+b,z) for a,b in arch_points(w,h,pointed)],thickness,mat)

def panel(name,x,y,z,w,h,mat=GLASS,pointed=False):
    # Closed thickness prevents single-sided FBX panels vanishing from one side.
    pts=arch_points(w,h,pointed);n=len(pts)
    verts=[(x+a,y+b,z+dz) for dz in [-.018,.018] for a,b in pts]
    faces=[tuple(range(n)),tuple(reversed(range(n,2*n)))]+[(i,(i+1)%n,(i+1)%n+n,i+n) for i in range(n)]
    return mesh_object(name,verts,faces,mat)

def window(name,x,y,z,w,h,pointed=False):
    panel(name+' luminous pane',x,y,z,w,h,GLASS,pointed)
    arch(name+' outer carved frame',x,y,z-.07,w+.2,h+.1,SLATE,.14,pointed)
    arch(name+' inset moulding',x,y,z-.12,w,h,CREAM,.06,pointed)
    spring=h*.55 if pointed else h-w/2
    for dx in [-w*.25,0,w*.25]:cube(name+' mullion',(x+dx,y+spring/2,z-.14),(.07,spring,.09),CREAM)
    for dy in [h*.24,spring]:cube(name+' transom',(x,y+dy,z-.14),(w,.08,.09),CREAM)
    cube(name+' sill',(x,y-.09,z-.09),(w+.4,.18,.38),SLATE,.035)
    if pointed:
        for side in [-1,1]:
            pipe(name+' tracery',[(x,y+spring,z-.14),(x+side*w*.22,y+spring+.5,z-.14),(x+side*w*.13,y+h-.45,z-.14),(x,y+h-.12,z-.14)],.055,CREAM)
    else:
        for a in [-.65,0,.65]:pipe(name+' fanlight',[(x,y+spring,z-.14),(x+math.sin(a)*w*.42,y+spring+math.cos(a)*w*.42,z-.14)],.04,CREAM)

def curtains(w,h):
    for side in [-1,1]:
        verts=[];faces=[];n=24
        for row,y in enumerate([0,h]):
            for j in range(n+1):
                t=j/n; x=side*(w*.32+t*w*.32)
                verts.append((x,y+.05*math.cos(t*math.pi*6)*(1-row),-.18-.11*math.cos(t*math.pi*8)))
        for j in range(n):faces.append((j,j+1,j+n+2,j+n+1))
        mesh_object('Pleated linen curtain',verts,faces,LINEN)
    pipe('Curtain pole',[(-w*.7,h+.12,-.2),(w*.7,h+.12,-.2)],.06,GOLD)
    for i in range(12):sphere('Curtain loop',((i/11-.5)*w*1.3,h+.1,-.2),(.07,.1,.07),GOLD,1)

def bed(variant):
    quilt=MINT if variant=='mint' else PINK
    for z,h in [(-1.6,1.1),(1.6,1.6)]:
        panel('Rounded wooden bed end',0,.15,z,2.15,h+.3,OAK)
        arch('Rounded bed end',0,.15,z-.1,2.15,h+.3,OAK,.12)
        for x in [-1,1]:cube('Bed turned leg',(x,.18,z),(.15,.36,.17),OAK,.04)
    cube('Bed frame',(0,.42,0),(2.1,.24,3.2),OAK,.07)
    sphere('Soft mattress',(0,.7,0),(1.0,.3,1.5),LINEN,3)
    sphere('Quilt drape',(0,.89,-.35),(1.04,.22,1.12),quilt,3)
    sphere('Ivory pillow',(0,1.02,1.05),(.79,.2,.38),LINEN,3)
    for x in [-.8,.8]:pipe('Quilt stitched edge',[(x,.96,-1.3),(x,1.08,-.3),(x,1.02,.65)],.018,CREAM)

def round_table(y=1.05,r=.65,cloth=False):
    cylinder('Round table top',(0,y,0),r,.12,CREAM if cloth else OAK,32)
    lathe('Turned pedestal',(0,0,0),[(0,.13),(.2,.22),(.4,.1),(.8,.1),(y-.06,.22)],OAK,n=16)
    for j in range(3):
        a=j*math.tau/3
        pipe('Curved table foot',[(0,.35,0),(.18*math.cos(a),.13,.18*math.sin(a)),(.52*math.cos(a),.07,.52*math.sin(a))],.07,OAK)
    if cloth:
        verts=[];faces=[]
        for row in range(2):
            for j in range(48):
                a=j*math.tau/48;rr=r*(1.01 if row==0 else 1.07)
                verts.append((rr*math.cos(a),y+.07 if row==0 else y-.22+.045*math.cos(a*10),rr*math.sin(a)))
        for j in range(48):k=(j+1)%48;faces.append((j,k,k+48,j+48))
        mesh_object('Scalloped tablecloth',verts,faces,LINEN)

def cup(x,y,z):
    lathe('Porcelain teacup',(x,y,z),[(0,.09),(.04,.12),(.2,.14),(.23,.145),(.23,.115),(.06,.09)],LINEN,n=16)
    cylinder('Tea',(x,y+.2,z),.112,.012,WOOD,16)
    pts=[(x+.14+.08*math.sin(t*math.pi),y+.12+.08*math.cos(t*math.pi),z) for t in [i/12 for i in range(13)]]
    pipe('Cup handle',pts,.025,LINEN,6)
    cylinder('Saucer',(x,y-.02,z),.21,.035,CREAM,24)

def chair():
    cube('Chair seat',(0,.62,0),(.72,.12,.7),OAK,.06)
    for x in [-.28,.28]:
        for z in [-.25,.25]:pipe('Chair leg',[(x*.95,.05,z*1.2),(x,.65,z)],.045,OAK)
    panel('Chair rounded back',0,.63,.27,.7,.83,OAK)
    arch('Chair back rim',0,.63,.27,.72,.86,CREAM,.055)
    for x in [-.22,0,.22]:pipe('Chair back grain',[(x,.76,.23),(x,.76+(.55-abs(x)*.5),.23)],.012,WOOD,5)

def books(x,y,z,n=9):
    for i in range(n):
        w=.15+random.random()*.09;h=.45+random.random()*.25
        cube('Bound book',(x+i*.24,y+h/2,z),(w,h,.39),BOOKS[i%len(BOOKS)],.015)
        for dy in [.1,h-.12]:cube('Book spine gilt',(x+i*.24,y+dy,z-.205),(w*.8,.025,.015),GOLD)

def bookshelf():
    cube('Bookcase back',(0,1.7,.25),(2.8,3.4,.13),GREEN)
    for x in [-1.34,1.34]:cube('Bookcase carved stile',(x,1.7,0),(.16,3.4,.65),CREAM,.04)
    for y in [.15,1.15,2.2,3.3]:
        cube('Bookcase shelf',(0,y,-.04),(2.8,.16,.72),CREAM,.035)
        if y<3:books(-1.13,y+.08,-.08,9)
    cube('Bookcase cornice',(0,3.48,-.04),(3.0,.2,.78),CREAM,.055)
    cylinder('Top vase',(1,3.8,0),.21,.5,TEAL,16,r2=.15)

def paper(x,y,z,rotation=0):
    o=cube('Loose parchment',(x,y,z),(.45,.012,.62),LINEN);o.rotation_euler.z=rotation
    for i in range(4):cube('Pencil writing',(x,y+.008,z-.18+i*.09),(.26,.004,.008),CARVE)

def desk():
    cube('Writing desk top',(0,1.2,0),(3,.16,1.4),CREAM,.05)
    for x in [-1.05,1.05]:
        cube('Drawer pedestal',(x,.58,0),(.73,1.16,1.1),CREAM,.04)
        for y in [.28,.68,.98]:
            cube('Drawer face',(x,y,-.56),(.63,.26,.035),LINEN,.025)
            pipe('Drawer brass pull',[(x-.12,y,-.61),(x,y-.04,-.66),(x+.12,y,-.61)],.025,GOLD,6)
    for i in range(7):cube('Stacked manuscripts',(-.75,1.32+i*.055,.05),(.72,.048,.7),LINEN,.01)
    books(.4,1.3,.2,3);paper(.05,1.3,-.2,.15)
    cylinder('Pencil jar',(1.15,1.5,-.25),.13,.38,OAK,12)
    for i in range(4):beam('Pencil',(1.1+i*.04,1.6,-.25),(1.07+i*.05,1.98,-.27),.012,WOOD)

def flowerpot():
    lathe('Terracotta flower pot',(0,0,0),[(0,.25),(.65,.4),(.7,.43),(.78,.43),(.79,.35)],CLAY,n=20)
    cylinder('Pot soil',(0,.73,0),.35,.025,EARTH,20)
    for i in range(7):
        a=i*2.39996;x=.29*math.cos(a);z=.29*math.sin(a);h=1.05+random.random()*.4
        beam('Flower stem',(0,.7,0),(x,h,z),.018,LEAF)
        for j in range(5):
            t=j*math.tau/5;sphere('Flower petal',(x+.1*math.cos(t),h,z+.1*math.sin(t)),(.095,.04,.095),ROSE if i%2 else GOLD,1)
        sphere('Flower center',(x,h+.025,z),(.045,)*3,LIGHT,1)
        sphere('Pot foliage',(x*.6,.9,z*.6),(.22,.07,.13),LEAVES[i%4],1)

def guardian():
    cube('Guardian pedestal',(0,.15,0),(1.25,.3,.8),SLATE,.06)
    for x in [-.24,.24]:
        sphere('Guardian shoe',(x,.37,-.12),(.2,.12,.3),SLATE)
        cylinder('Guardian leg',(x,.62,0),.15,.5,SLATE,12)
    sphere('Guardian tunic',(0,1.13,0),(.42,.5,.27),SLATE)
    sphere('Guardian hair cap',(0,2.02,.04),(.63,.6,.36),CARVE,3)
    sphere('Guardian face',(0,2.0,-.19),(.52,.48,.29),SLATE,3)
    for x in [-.38,-.18,.05,.28]:sphere('Sculpted fringe',(x,2.31,-.36),(.22,.2,.11),CARVE)
    for side in [-1,1]:
        sphere('Guardian long hair',(side*.52,1.57,.06),(.15,.55,.2),CARVE)
        pipe('Carved closed eye',[(side*.17-.09,2.04,-.46),(side*.17,2.0,-.49),(side*.17+.09,2.04,-.46)],.018,CARVE,6)
        sphere('Guardian arm',(side*.43,1.3,-.1),(.15,.36,.16),SLATE)
        for i in range(3):sphere('Guardian wing feather',(side*(.48+i*.11),1.2+i*.14,.17),(.15,.37,.09),SLATE)
    pipe('Guardian ceremonial sword',[(-.38,.63,-.35),(.35,1.52,-.44)],.055,CARVE)
    cube('Guardian clasp',(0,1.4,-.3),(.22,.2,.09),CREAM,.03)
    arch('Guardian niche arch',0,0,.37,1.7,3.15,CARVE,.13,True)

def living_tree(name,h,r):
    bark=OAK
    lathe(name+' twisted trunk',(0,0,0),[(0,r*.28),(h*.12,r*.2),(h*.32,r*.16),(h*.55,r*.18),(h*.72,r*.06)],bark,n=18)
    for j in range(10):
        a=j*math.tau/10
        pipe(name+' bark ridge',[(math.cos(a)*r*.27,0,math.sin(a)*r*.27),(math.cos(a+.3)*r*.17,h*.32,math.sin(a+.3)*r*.17),(math.cos(a+.5)*r*.17,h*.55,math.sin(a+.5)*r*.17)],.035,WOOD,5)
    for j in range(7):
        a=j*2.39996;x=math.cos(a)*r*.62;z=math.sin(a)*r*.62;y=h*(.72+random.random()*.15)
        pipe(name+' branching bough',[(0,h*.4,0),(x*.4,h*.6,z*.4),(x,y,z)],r*.065,bark,9)
        sphere(name+' dense foliage core',(x,y,z),(r*.54,h*.18,r*.5),LEAVES[j%4],2)
        batches=[([],[]) for _ in range(4)]
        for k in range(170):
            a2=random.random()*math.tau;t=random.uniform(-1,1);rad=(1-t*t)**.5
            px=x+math.cos(a2)*rad*r*.56;py=y+t*h*.19;pz=z+math.sin(a2)*rad*r*.52
            scale=.12+r*.035;ang=random.random()*math.tau;u=(math.cos(ang)*scale,math.sin(ang)*scale)
            vs,fs=batches[k%4];i=len(vs)
            vs.extend([(px-u[0],py,pz-u[1]),(px-u[1]*.5,py+.04,pz+u[0]*.5),(px+u[0],py,pz+u[1]),(px+u[1]*.5,py-.02,pz-u[0]*.5),(px,py+.08,pz)])
            fs.extend([(i,i+1,i+4),(i+1,i+2,i+4),(i+2,i+3,i+4),(i+3,i,i+4),(i+3,i+2,i+1,i)])
        for k,(vs,fs) in enumerate(batches):mesh_object(name+' leaf clusters',vs,fs,LEAVES[k])

def oval_rug(variant):
    mat=MINT if variant=='mint' else GREEN
    lathe('Oval woven rug',(0,0,0),[(0,1),(.025,1)],mat,rx=2.5,rz=1.7,n=48)
    for rad in [.82,.91]:pipe('Rug woven border',[(math.cos(a)*2.5*rad,.03,math.sin(a)*1.7*rad) for a in [i*math.tau/64 for i in range(65)]],.025,GOLD)
    for j in range(6):
        a=j*math.tau/6
        sphere('Rug floral motif',(.7*math.cos(a),.035,.5*math.sin(a)),(.35,.007,.18),GOLD,2)

def throne():
    for x in [-.75,.75]:
        for z in [-.4,.4]:cube('Throne gilded leg',(x,.3,z),(.16,.6,.16),GOLD,.035)
    cube('Throne seat frame',(0,.67,0),(1.8,.2,1.15),GOLD,.07)
    sphere('Red velvet seat',(0,.82,-.03),(.79,.17,.49),RED,3)
    panel('Arched throne gold back',0,.72,.48,1.8,2.1,GOLD)
    panel('Arched red upholstery',0,.83,.425,1.53,1.82,RED)
    arch('Throne rolled frame',0,.72,.39,1.8,2.1,GOLD,.12)
    for x in [-.9,.9]:
        cube('Throne armrest',(x,1.24,-.02),(.18,.2,1.03),GOLD,.065)
        cube('Throne arm cushion',(x,1.36,-.02),(.17,.05,.84),RED,.015)
        cylinder('Throne arm support',(x,.94,-.4),.07,.6,GOLD,12)
    for x in [-.4,0,.4]:
        for y in [1.15,1.55,1.95,2.3]:sphere('Velvet tuft button',(x,y,.39),(.035,.035,.018),GOLD,1)
    crown('Throne crown',0,2.8,.46,.45)

def gabled_roof(name,x,y,z,w,d,red=False):
    # Swept tile rows follow an upturned storybook eave, rather than a flat cone.
    tilemats=RED_TILES if red else ROOF_TILES
    profile=[(-.55*w,y+.1),(-.48*w,y+.25),(-.34*w,y+.9),(-.18*w,y+2.3),(0,y+3.5),(.18*w,y+2.3),(.34*w,y+.9),(.48*w,y+.25),(.55*w,y+.1)]
    verts=[(x+xx,yy,z+dz) for dz in [-d/2-.45,d/2+.45] for xx,yy in profile];n=len(profile)
    fs=[(i,i+1,i+1+n,i+n) for i in range(n-1)]
    mesh_object(name+' curved roof substrate',verts,fs,tilemats[0])
    # Cream triangular gable faces and brown bargeboards.
    for zz in [z-d/2-.3,z+d/2+.3]:
        mesh_object(name+' plaster gable',[(x+xx,yy-.08,zz) for xx,yy in profile]+[(x+w*.55,y,zz),(x-w*.55,y,zz)],[tuple(range(n+2))],CREAM)
        pipe(name+' carved eave border',[(x+xx,yy+.08,zz) for xx,yy in profile],.15,WOOD)
        pipe(name+' gable kingpost',[(x,y,zz-.03),(x,y+3.45,zz-.03)],.1,WOOD)
    batches=[([],[]) for _ in tilemats]
    for side in [-1,1]:
        for row in range(8):
            t0=row/8*.5*w;t1=(row+1)/8*.5*w
            def height(t):
                xp=side*t
                for (a,ya),(b,yb) in zip(profile,profile[1:]):
                    if a<=xp<=b:return ya+(yb-ya)*(xp-a)/(b-a)
                return y+.2
            for col in range(max(1,int(d/.8))):
                zz=z-d/2+col*.8+.4+(row%2)*.1;half=.39
                vs,fs=batches[(row+col)%len(batches)];i=len(vs)
                vs.extend([(x+side*t0,height(t0)+.07,zz-half),(x+side*t0,height(t0)+.07,zz+half),(x+side*t1,height(t1)+.08,zz+half*.85),(x+side*(t1+.08),height(t1)+.055,zz),(x+side*t1,height(t1)+.08,zz-half*.85)])
                fs.append(tuple(i+j for j in range(5)))
    for mat,(vs,fs) in zip(tilemats,batches):mesh_object(name+' individual scalloped shingles',vs,fs,mat)
    pipe(name+' ridge cap',[(x,y+3.58,z-d/2-.55),(x,y+3.58,z+d/2+.55)],.12,OAK)

def ivy(name,x,y,z,length=2):
    for j in range(4):
        xx=x+(j-1.5)*.22
        pts=[(xx+.12*math.sin(t*5+j),y-t*length,z-.02) for t in [i/12 for i in range(13)]]
        pipe(name+' vine',pts,.015,LEAF,5)
        for i,(a,b,c) in enumerate(pts):
            sphere(name+' heart leaf',(a+(.09 if i%2 else -.09),b,c-.025),(.13,.18,.035),LEAVES[(j+i)%4],1)

def make_prop(prop):
    kind=prop['kind'];variant=prop.get('variant','')
    if kind=='bed':bed(variant)
    elif kind=='chair':chair()
    elif kind=='bookshelf':bookshelf()
    elif kind=='desk':desk()
    elif kind=='flowerpot':flowerpot()
    elif kind=='guardian':guardian()
    elif kind=='throne':throne()
    elif kind=='oval_rug':oval_rug(variant)
    elif kind=='arched_window':
        window('Bedroom arched window',0,0,0,3.6,3.4)
        if variant=='curtain':curtains(3.6,3.5)
    elif kind in ['cafe_table','bedside','royal_side_table']:
        round_table(1.05,.72 if kind=='cafe_table' else .48,kind=='cafe_table')
        if kind=='bedside':
            sphere('Mint plush bear body',(-.15,1.36,0),(.15,.21,.12),MINT)
            sphere('Mint plush bear head',(-.15,1.65,0),(.18,.16,.13),MINT)
            for x in [-.25,-.05]:sphere('Plush seated foot',(x,1.13,-.1),(.1,.08,.12),MINT)
            for x in [-.33,.03]:sphere('Plush rounded arm',(x,1.33,0),(.07,.13,.07),MINT)
            for x in [-.28,-.02]:sphere('Plush bear ear',(x,1.8,0),(.07,)*3,MINT)
            for x in [-.21,-.09]:sphere('Plush bear eye',(x,1.66,-.12),(.018,)*3,WOOD,1)
            lathe('Bell jar',(0.25,1.12,.1),[(0,.15),(.32,.15),(.43,.08),(.46,0.01)],GLASS,n=16)
        else:
            cup(-.25,1.13,-.1)
            if kind=='cafe_table':
                cylinder('Dessert plate',(.25,1.13,.12),.24,.03,LINEN,24)
                for dx in [-.08,.08]:sphere('Strawberry cake',(.25+dx,1.23,.12),(.08,.09,.08),RED,2)
            else:
                cylinder('Lemonade glass',(.2,1.27,.08),.09,.28,LIGHT,16)
                pipe('Drink straw',[(.2,1.28,.08),(.2,1.6,.08),(.31,1.63,.08)],.012,RED,5)
    elif kind=='satchel':
        cube('Green school bag',(0,.42,0),(.7,.8,.34),TEAL,.11)
        cube('Satchel front pocket',(0,.3,-.2),(.52,.35,.12),MINT,.07)
        pipe('Satchel handle',[(-.15,.8,0),(-.13,.99,0),(.13,.99,0),(.15,.8,0)],.04,WOOD)
        cylinder('Satchel clasp',(0,.65,-.23),.08,.04,GOLD,12)
    elif kind=='menu_board':
        for x in [-.42,.42]:
            beam('Menu A frame',(x,0,-.25),(x,1.75,.12),.045,OAK)
            beam('Menu rear leg',(x,0,.32),(x,1.75,.12),.045,OAK)
        cube('Menu chalk panel',(0,.95,-.13),(.77,1.22,.09),INK,.025)
        for y in [.6,.78,.98,1.2]:cube('Chalk menu line',(0,y,-.185),(.45,.025,.005),LINEN)
        cup(0,1.5,.05)
    elif kind=='cafe_counter':
        cube('Cafe counter',(0,.6,0),(7,1.2,1.3),OAK,.08)
        cube('Cafe counter marble top',(0,1.26,0),(7.2,.14,1.5),CREAM,.045)
        for x in [-2.8,-1.4,0,1.4,2.8]:
            cube('Counter panel',(x,.65,-.67),(1.2,.8,.04),GREEN,.035)
            cup(x,1.35,-.2)
        books(-2,1.35,.25,4)
        lathe('Cafe kettle',(2,1.35,.1),[(0,.2),(.2,.3),(.4,.23),(.45,.14),(.5,.04)],TEAL,n=20)
    elif kind=='study_arch':arch('Carved study arch',0,0,0,4,5.35,CREAM,.18)
    elif kind=='pinboard':
        cube('Study pinboard',(0,0,0),(2.5,1.6,.07),GREEN,.025)
        for i in range(7):
            x=(i%4-.9)*.53;y=(i//4-.5)*.64
            cube('Pinned manuscript',(x,y,-.05),(.4,.5,.015),LINEN)
            sphere('Brass paper pin',(x,y+.2,-.075),(.035,)*3,GOLD,1)
    elif kind=='wall_frame':
        cube('Picture frame',(0,0,0),(1.45,1.1,.12),OAK,.035)
        cube('Picture paper',(0,0,-.08),(1.25,.9,.02),LINEN)
        if variant=='tree':
            cube('Picture trunk',(0,-.12,-.1),(.08,.4,.01),WOOD)
            sphere('Picture tree canopy',(0,.15,-.11),(.4,.23,.008),LEAF2,2)
        else:crown('Framed crown',0,-.1,-.12,.28)
    elif kind in ['village_tree','atrium_tree']:
        r=4.2 if kind=='village_tree' else 1.6
        lathe('Circular white planter',(0,0,0),[(0,r),(.18,r),(.22,r-.15)],SLATE,n=64)
        cylinder('Circular grassy bed',(0,.23,0),r-.2,.035,LEAVES[2],64)
        living_tree('Village oak' if kind=='village_tree' else 'Atrium sacred tree',10.5 if kind=='village_tree' else 8.5,4.6 if kind=='village_tree' else 2.1)
        for i in range(30):
            a=i*2.39996;rr=r*.7+random.random()*r*.2
            sphere('Planter flowers',(math.cos(a)*rr,.3,math.sin(a)*rr),(.1,.12,.1),GOLD if i%3 else LINEN,1)
    elif kind=='clock_plinth':
        cube('Clock plinth',(0,.25,0),(1.4,.5,1),SLATE,.12)
        sphere('Village golden clock',(0,.86,0),(.61,.67,.24),GOLD,3)
        sphere('Clock face',(0,.9,-.23),(.43,.43,.025),CREAM,3)
        pipe('Clock hands',[(.22,.9,-.27),(0,.9,-.27),(0,1.2,-.27)],.025,WOOD,6)
    elif kind=='street_lamp':lamp('Village lantern',0,0)
    elif kind=='sconce':
        cube('Sconce wall plate',(0,0,.05),(.22,.5,.12),CARVE,.03)
        pipe('Sconce bracket',[(0,-.15,0),(0,-.25,-.35),(0,.05,-.45)],.06,CARVE)
        lathe('Sconce cup',(0,.05,-.45),[(0,.12),(.23,.23),(.3,.24)],CARVE,n=16)
        sphere('Sconce pearl',(0,.36,-.45),(.13,.19,.13),LIGHT,2)
    elif kind=='altar':
        for y,w in [(.12,1.7),(.35,1.4),(1.1,1.3),(1.35,1.75)]:cube('Hall altar moulding',(0,y,0),(w,.2,.9),SLATE,.045)
        cube('Altar stem',(0,.74,0),(1.1,.7,.7),SLATE,.05)
