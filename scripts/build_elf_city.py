"""Blender 5.x authoring and export. Run with -- --render for contact previews.

Named editable collections, original geometry, collision skin, spatial/material
batches and deterministic seed. The city hall banner uses sourced Elena art.
"""
import bpy, bmesh, json, math, random, sys
from pathlib import Path
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'scripts'))
from elf_city_layout import make_layout, OUT
from map_lettering import add_direction_arrow, font_body
from elf_city_banner import add_banner
OUT.mkdir(parents=True,exist_ok=True)
ART=ROOT/'artifacts/elf-city';ART.mkdir(parents=True,exist_ok=True)
layout=make_layout()
(OUT/'layout.json').write_text(json.dumps(layout,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
random.seed(413)
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
if hasattr(bpy.context.preferences.filepaths,'file_preview_type'):
    bpy.context.preferences.filepaths.file_preview_type='NONE'
COL=None
def section(name):
    global COL
    COL=bpy.data.collections.new(name);bpy.context.scene.collection.children.link(COL)
    print('ELF_SECTION',name,flush=True)
def linear(v): return v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4
def material(name,c,metal=0,glow=0):
    m=bpy.data.materials.new(name);m.use_nodes=True;m['srgb']=c
    rgba=tuple(linear(v/255) for v in c)+(1,)
    m.diffuse_color=rgba
    p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=rgba
    p.inputs['Roughness'].default_value=.65;p.inputs['Metallic'].default_value=metal
    if glow:
        p.inputs['Emission Color'].default_value=rgba;p.inputs['Emission Strength'].default_value=glow
    return m
NAVY=material('Navy',[55,70,98]);SLATE=material('Slate',[80,102,125])
TEAL=material('Teal',[39,126,135]);PALE=material('Pearl',[157,178,187],.2)
BLACK=material('Ink',[18,29,48]);ROAD=material('Asphalt',[43,57,78]);PAVE=material('Paving',[75,91,113])
WOOD=material('Wood',[114,75,69]);LEAF=material('Leaves',[32,92,83]);LEAF2=material('LeavesMint',[46,116,99])
PLUM=material('Plum',[113,60,97]);GLASS=material('Glass',[26,62,83],.35)
CYAN=material('GlowCyan',[80,235,231],glow=3)
GOLD=material('GlowGold',[255,207,112],glow=3)
PINK=material('GlowPink',[244,106,174],glow=2)
WHITE=material('GlowWhite',[184,225,246],glow=2)
RED=material('GlowRed',[255,71,110],glow=2)
MATS={tuple(m['srgb']):m for m in [NAVY,SLATE,TEAL,PALE]}
def loc(p):return (p[0],-p[2],p[1])
def mesh_obj(name,mesh,p,mat):
    o=bpy.data.objects.new(name,mesh);COL.objects.link(o);o.location=loc(p)
    mesh.materials.append(mat);return o
def box(name,p,s,mat,bevel=0):
    mesh=bpy.data.meshes.new(name)
    mesh.from_pydata([(x*s[0]/2,y*s[2]/2,z*s[1]/2) for x,y,z in [(-1,-1,-1),(-1,-1,1),(-1,1,-1),(-1,1,1),(1,-1,-1),(1,-1,1),(1,1,-1),(1,1,1)]],[],
                     [(0,4,6,2),(1,3,7,5),(0,1,5,4),(2,6,7,3),(0,2,3,1),(4,5,7,6)])
    if bevel:
        bm=bmesh.new();bm.from_mesh(mesh);bmesh.ops.bevel(bm,geom=list(bm.edges),offset=min(bevel,min(s)*.23),segments=2,affect='EDGES');bm.to_mesh(mesh);bm.free()
    return mesh_obj(name,mesh,p,mat)
def sphere(name,p,s,mat):
    mesh=bpy.data.meshes.new(name);bm=bmesh.new();bmesh.ops.create_icosphere(bm,subdivisions=2,radius=1)
    bm.to_mesh(mesh);bm.free();o=mesh_obj(name,mesh,p,mat);o.scale=(s[0],s[2],s[1]);return o
def cylinder(name,p,r,h,mat,n=12,r2=None):
    mesh=bpy.data.meshes.new(name);bm=bmesh.new()
    bmesh.ops.create_cone(bm,cap_ends=True,cap_tris=False,segments=n,radius1=r,radius2=r if r2 is None else r2,depth=h)
    bm.to_mesh(mesh);bm.free();return mesh_obj(name,mesh,p,mat)
def beam(name,a,b,r,mat):
    av,bv=Vector(loc(a)),Vector(loc(b));o=cylinder(name,(0,0,0),r,(bv-av).length,mat,8)
    o.location=(av+bv)/2;o.rotation_euler=(bv-av).to_track_quat('Z','Y').to_euler();return o
font=bpy.data.fonts.load(str(ROOT/'assets/fonts/Hungeul.otf'))
def text(name,body,p,size,mat,angle=math.pi):
    curve=bpy.data.curves.new(name,'FONT');curve.body=font_body(body);curve.font=font;curve.size=size
    curve.align_x='CENTER';curve.align_y='CENTER';curve.extrude=.008;curve.resolution_u=3
    o=bpy.data.objects.new(name,curve);COL.objects.link(o);o.location=loc(p)
    o.rotation_euler=(math.pi/2,0,angle);o.data.materials.append(mat)
    bpy.ops.object.select_all(action='DESELECT');o.select_set(True);bpy.context.view_layer.objects.active=o
    bpy.ops.object.convert(target='MESH');o=bpy.context.object
    add_direction_arrow(o.data,body,size)
    o['map_text']=body;o['map_font']='assets/fonts/Hungeul.otf';o['map_font_size']=size
    return o
def ring(name,x,y,z,r,mat):
    for j in range(24):
        a=j*math.tau/24;b=(j+1)*math.tau/24
        beam(name,(x+r*math.cos(a),y+r*math.sin(a),z),(x+r*math.cos(b),y+r*math.sin(b),z),r*.07,mat)
def emblem(x,y,z,s=1):
    ring('Municipal gear insignia',x,y,z,s,CYAN)
    for j in range(10):
        a=j*math.tau/10
        box('Gear tooth',(x+math.cos(a)*s,y+math.sin(a)*s,z),(.24*s,.24*s,.09),CYAN)
    beam('Left elf ear',(x-.15*s,y+.2*s,z),(x-.53*s,y+.67*s,z),.09*s,PALE)
    beam('Right elf ear',(x+.15*s,y+.2*s,z),(x+.53*s,y+.67*s,z),.09*s,PALE)
    box('Civic core',(x,y-.1*s,z),(.45*s,.5*s,.08),PALE,.05)
def lamp(x,z,y=0):
    cylinder('Lamp base',(x,y+.13,z),.27,.26,SLATE)
    cylinder('Lamp shaft',(x,y+1.8,z),.07,3.4,PALE,8)
    cylinder('Lantern glass',(x,y+3.55,z),.27,.55,GOLD,6)
    cylinder('Lantern canopy',(x,y+3.94,z),.4,.3,TEAL,6,0)
    for dx in [-.2,.2]:beam('Lamp cage',(x+dx,y+3.3,z),(x+dx,y+3.8,z),.025,BLACK)
def tree(x,z,y=0,s=1):
    cylinder('Tree trunk',(x,y+1.5*s,z),.22*s,3*s,WOOD,9,r2=.14*s)
    for dx,dz,dy,r in [(0,0,3.7,1.7),(-1,.1,3.3,1.2),(.8,.5,3.6,1.3),(.1,-.8,4.2,1.2)]:
        beam('Tree branch',(x,y+2*s,z),(x+dx*s,y+dy*s,z+dz*s),.1*s,WOOD)
        sphere('Leaf crown',(x+dx*s,y+dy*s,z+dz*s),(r*s,r*.8*s,r*s),random.choice([LEAF,LEAF2]))
def bench(x,z,angle=0):
    parts=[]
    for i in range(4):parts.append(box('Bench seat',(x,.62,z+i*.13),(2.4,.11,.1),WOOD,.02))
    for i in range(3):parts.append(box('Bench back',(x,.9+i*.19,z+.45),(2.4,.13,.1),WOOD,.02))
    for dx in [-.85,.85]:
        parts.append(box('Bench leg',(x+dx,.32,z+.2),(.12,.64,.55),SLATE))
    if angle:
        pivot=Vector(loc((x,0,z)))
        from mathutils import Matrix
        rot=Matrix.Rotation(angle,4,'Z')
        for o in parts:o.location=pivot+rot.to_3x3()@(o.location-pivot);o.rotation_euler.z=angle
def vending(x,z):
    box('Vending cabinet',(x,1.25,z),(1.4,2.5,.85),PALE,.14)
    box('Vending dark bezel',(x,1.35,z-.44),(1.12,1.6,.07),BLACK,.05)
    for a in range(3):
        for b in range(4):box('Block ration selection',(x-.32+a*.32,.85+b*.31,z-.49),(.2,.19,.03),[CYAN,GOLD,PINK][a],.03)
    box('Vending collection slot',(x,.33,z-.47),(.78,.2,.04),BLACK)
    text('Vending header','BLOCK 24',(x,2.27,z-.5),.17,CYAN)
def rail(name,a,b,y):
    beam(name+' top',(a[0],y+.9,a[1]),(b[0],y+.9,b[1]),.04,PALE)
    length=math.dist(a,b);count=max(1,int(length/1.6))
    for i in range(count+1):
        t=i/count;x=a[0]+(b[0]-a[0])*t;z=a[1]+(b[1]-a[1])*t
        beam(name+' post',(x,y,z),(x,y+.9,z),.04,SLATE)

section('01 Shared collision architecture')
for b in layout['blocks']:
    if not b.get('render',True):continue
    o=box(b['name'],b['center'],b['size'],MATS.get(tuple(b['color']),NAVY),.045 if 'step' not in b['name'] else 0)
    o['collision']=True;o['phaseable']=b['phaseable']
box('City foundation',(23,-.5,-8),(150,.7,160),NAVY)
box('Playable paving',(23,-.035,-8),(90,.07,92),PAVE)
box('Approach asphalt',(-1,-.005,-30),(17,.035,32),ROAD)
box('Civic dark promenade',(35,-.005,14),(8,.035,39),ROAD)
for x in range(-22,69,3):box('Paving joint',(x,.007,-8),(.027,.014,92),NAVY)
for z in range(-54,39,3):box('Paving joint',(23,.009,z),(90,.014,.027),NAVY)
for z in range(-41,-17,4):box('Dashed lane marking',(.5,.025,z),(.12,.025,1.6),PALE)
for x in range(-6,8,2):box('Crosswalk stripe',(x,.027,-39),(1,.025,2.6),PALE)

section('02 Skyline and facade language')
def tower(name,x,z,w,d,h,mat=NAVY):
    box(name,(x,h/2,z),(w,h,d),mat,.3)
    box(name+' rooftop',(x,h+.2,z),(w+.6,.4,d+.6),SLATE,.1)
    for y in range(3,int(h),3):
        box('Horizontal facade band',(x,y,z-d/2-.04),(w+.15,.14,.1),SLATE)
        box('Vertical side band',(x-w/2-.04,y,z),(.1,.14,d+.15),SLATE)
        for dx in range(1,int(w)-1,2):
            c=random.choices([GOLD,GLASS,CYAN],[.48,.47,.05])[0]
            box('Night window',(x-w/2+dx,y+1,z-d/2-.08),(.75,1.35,.07),c)
            box('Rear night window',(x-w/2+dx,y+1,z+d/2+.08),(.75,1.35,.07),random.choice([GOLD,GLASS]))
        for dz in range(1,int(d)-1,2):
            box('Side night window',(x-w/2-.08,y+1,z-d/2+dz),(.07,1.35,.75),random.choice([GOLD,GLASS,GLASS]))
            box('Side night window',(x+w/2+.08,y+1,z-d/2+dz),(.07,1.35,.75),random.choice([GOLD,GLASS]))
    box('Roof machinery',(x,h+1,z),(w*.35,2,d*.3),BLACK,.15)
    cylinder('Roof antenna',(x+w*.3,h+1.7,z),.045,3.2,SLATE,6)
    sphere('Red aircraft beacon',(x+w*.3,h+3.35,z),(.13,.18,.13),RED)
for i,(x,z,w,d,h) in enumerate([(-31,-45,13,12,38),(-32,-25,15,17,50),(-33,-3,15,13,32),(-30,18,16,15,43),
    (-19,49,15,14,36),(2,48,18,12,49),(23,49,13,12,37),(43,51,15,12,57),(66,49,15,12,42),
    (80,25,15,13,35),(82,3,16,16,48),(82,-23,13,17,41),(77,-47,18,13,58),
    (45,-48,14,10,39),(29,-42,12,12,32),(59,-29,13,13,45)]):tower('Skyline tower %02d'%i,x,z,w,d,h)
# Panel the research mass and hangar; avoid entrances with shallow applied details.
for x in range(23,66,3):
    box('Research north window',(x,14,-9.94),(1.2,5,.08),GLASS)
    box('Research roof rib',(x,24.2,-32),(.18,.4,43),SLATE)
for z in range(-50,-11,4):
    box('Research west window',(20.44,15,z),(.08,4.8,1.4),random.choice([GOLD,GLASS]))
for x in [-9,6,21]:
    box('Hangar entry buttress',(x,4.5,-17.3),(.65,9,1),SLATE,.1)
box('Transit entry fascia',(6,7.2,-17.35),(27,2.4,.32),BLACK,.15)
text('Transit hall sign','모나티엄   환승 정비고',(6,7.3,-17.54),1.0,CYAN)
text('Transit subtitle','MONATIUM   /   TRANSIT 01',(6,6.55,-17.55),.28,PALE)
for z in [-15,-8,-1,6]:
    beam('Roof truss',(-10,9,z),(6,11.6,z),.16,SLATE)
    beam('Roof truss',(6,11.6,z),(22,9,z),.16,SLATE)
    beam('Roof tie',(-10,9,z),(22,9,z),.08,SLATE)
    box('Skylight luminous rib',(6,11.45,z),(26,.08,.13),CYAN)
for x in [-8,20]:box('Hall ceiling strip',(x,8.9,-3.5),(.12,.07,25),WHITE)

section('03 Elevated line and parked monorail')
# Skyline transit is above the playable deck; no invisible obstacle in walking space.
box('Monorail viaduct',(38,12,16),(5.5,.9,110),SLATE,.2)
for x in [35.5,40.5]:
    box('Viaduct luminous edge',(x,12.5,16),(.13,.12,110),CYAN)
    box('Viaduct parapet',(x,12.9,16),(.22,.8,110),PALE,.06)
for z in [-39,-18,3,26,48]:
    # Support columns in research mass or planted islands / outside playable area.
    if z==3: continue
    if z==26: x=36
    else:x=38
    box('Viaduct support',(x,6,z),(1.3,12,1.8),SLATE,.1)
# Docked vehicle's blocking hull is shared with the server. Thin shell details.
box('Monorail roof',(6,3.64,-5),(4.55,.08,12.9),PALE,.035)
for x in [3.66,8.34]:
    box('Train teal stripe',(x,1.15,-5),(.07,.32,12),TEAL)
    for z in [-10,-7,-4,-1]:box('Train window',(x,2.5,z),(.09,1.1,1.9),GLASS,.08)
box('Train windshield',(6,2.4,-11.54),(3.6,1.3,.1),GLASS,.1)
for x in [4.4,7.6]:box('Monorail headlamp',(x,.9,-11.57),(.48,.2,.08),CYAN,.04)
text('Train serial','M-041',(6,1.6,-11.6),.36,TEAL)
for z in [-9,-2]:
    for x in [3.75,8.25]:cylinder('Magnetic bogie',(x,.4,z),.36,.5,BLACK,12)
rail('Overpass south',(-14,-27.5),(14,-27.5),3.6)
rail('Overpass north',(-14,-24.5),(14,-24.5),3.6)
rail('Hangar gallery',(-8, -5),(-8,7.5),3.6)
rail('Train deck',(-4,4.4),(8.7,4.4),3.6)

section('04 Civic square and municipal architecture')
# Hall faces west; rotate the front details as a group around the civic building.
before=set(bpy.data.objects)
box('Civic entrance portal',(0,4.5,0),(10,9,.5),BLACK,.15)
box('Civic glowing doorway',(0,2.1,-.3),(3.3,4.2,.08),GOLD,.08)
for x in [-4.5,-3.9,3.9,4.5]:box('Civic fluted pier',(x,5.4,-.45),(.25,10.8,.35),PALE,.04)
for x in [-2.8,-2.2,2.2,2.8]:box('Civic crown light',(x,10.7,-.4),(.18,3.7,.2),GOLD)
text('Civic title','모나티엄 시청',(0,5.25,-.5),.53,PALE)
from mathutils import Matrix
rot=Matrix.Rotation(math.pi/2,4,'Z')
for o in set(bpy.data.objects)-before:
    o.location=Vector(loc((55.9,0,18)))+rot.to_3x3()@o.location
    o.rotation_euler.z+=math.pi/2
add_banner()
for z in [11,13,15,21,23,25]:
    box('City hall outer rib',(55.85,7,z),(.4,11,.18),PALE)
    box('City hall lower windows',(55.7,1.7,z),(.08,2.1,1.1),GOLD)
for x,z in [(36,24),(45,29),(43,14),(30,7),(-19,-39),(17,-20),(30,32)]:tree(x,z,1.1,.85)
for x,z in [(32,27),(41,29),(42,12),(-20,-36),(-18,-15),(31,5),(51,27),(48,10),(28,30)]:lamp(x,z)
for x,z in [(39,29),(48,29),(39,12),(30,27)]:bench(x,z)
for x,z in [(51,6.3),(-17,-36),(29,5)]:vending(x,z)
box('Cafe sign frame',(49,6,5.6),(12,1.5,.2),BLACK,.1)
text('Cafe neon sign','밤샘 연구소  /  COFFEE',(49,6,5.74),.64,PINK,0)
for x in [42,46,50,54,57]:box('Cafe warm storefront',(x,2.3,5.54),(2.4,3.5,.08),GOLD)

section('05 Mint PC cafe interior')
box('Cafe glossy tile floor',(-15,.018,-28),(10.5,.025,17.4),TEAL)
for z in range(-36,-19,2):box('Cafe floor seam',(-15,.04,z),(10,.015,.035),BLACK)
for x in [-19,-17,-15,-13,-11]:box('Cafe floor seam',(x,.04,-28),(.035,.015,17),BLACK)
box('Cafe illuminated fascia',(-15,4.4,-37.29),(10,1.2,.2),TEAL,.08)
text('PC cafe sign','민트 PC  /  24H',(-15,4.45,-37.42),.7,CYAN)
text('PC cafe wayfinding','측면 통로  →',(-15,3.7,-37.44),.25,PALE)
def workstation(x,z,side=1):
    box('Workstation desk',(x,.9,z),(1.4,.12,1.05),WOOD,.04)
    for dx in [-.55,.55]:box('Workstation leg',(x+dx,.42,z),(.08,.84,.7),PALE)
    box('Monitor frame',(x,1.55,z+.28),(1.18,.8,.15),BLACK,.09)
    box('Mint monitor',(x,1.56,z+.19),(1,.63,.035),CYAN,.03)
    for n in range(3):box('Terminal text',(x-.15,1.4+n*.15,z+.165),(.55,.035,.015),TEAL)
    box('Monitor foot',(x,1.12,z+.28),(.1,.35,.12),SLATE)
    box('Keyboard',(x,.99,z-.23),(.75,.05,.28),BLACK,.02)
    box('Mouse',(x+.5,1,z-.23),(.12,.06,.2),PALE,.025)
    box('PC tower',(x+.5,.4,z+.1),(.3,.7,.5),BLACK,.03)
    for y in [.3,.55]:ring('RGB fan',x+.5,y,z-.16,.09,PINK)
    box('Gaming chair cushion',(x,.55,z-1.05),(.68,.17,.65),BLACK,.1)
    box('Gaming chair back',(x,1.1,z-1.35),(.7,1,.16),TEAL,.1)
    cylinder('Chair stem',(x,.25,z-1.05),.06,.5,SLATE)
    for dx in [-.4,.4]:beam('Chair feet',(x,.12,z-1.05),(x+dx,.1,z-1.25),.035,SLATE)
for z in [-33,-29,-25]:
    for x in [-18,-12]:workstation(x,z)
for x in [-19,-11]:
    box('Ceiling pipe',(x,5.5,-28),(.22,.22,17),TEAL,.05)
    box('Cafe linear light',(x,5.2,-28),(.1,.07,16),CYAN)
box('Air conditioner',(-15,5.65,-27),(2.4,.4,2),PALE,.1)
for x in [-15.8,-15.4,-15,-14.6,-14.2]:box('AC slats',(x,5.42,-27),(.1,.05,1.6),BLACK)
emblem(-18,3.4,-19.29,.7)

section('06 Night shift studio and service alley')
for x,z in [(23,20),(25,20)]:workstation(x,z)
box('Studio bed frame',(28, .35,20),(2.2,.7,3.6),WOOD,.09)
box('Studio mattress',(28,.8,20),(2.1,.35,3.5),PALE,.1)
box('Studio plum duvet',(28,1.02,19.5),(2,.18,2.3),PLUM,.1)
box('Studio pillow',(28,1.1,21.1),(1.4,.3,.75),PALE,.1)
for x in [22,23]:
    box('Server rack',(x,1.4,12),( .75,2.8,1),BLACK,.06)
    for y in [.3,.65,1,1.35,1.7,2.05,2.4]:box('Server status strip',(x,y,11.49),(.58,.07,.025),CYAN)
box('Studio sign panel',(25.5,4.4,9.73),(7,1,.1),BLACK)
text('Studio neon','NIGHT SHIFT / 041',(25.5,4.45,9.65),.43,PINK)
for x,z in [(-19,-10),(-18,-8),(23.8,26),(25.2,25.5)]:
    cylinder('Service trash bin',(x,.6,z),.4,1.2,SLATE,10)
    cylinder('Trash bin lid',(x,1.24,z),.45,.13,BLACK,10)
    sphere('Bagged recyclables',(x+.65,.3,z),(.5,.35,.4),BLACK)
    box('Discarded carton',(x-.7,.25,z+.3),(.6,.5,.6),WOOD,.04)
for z in [-15,-12,-9,-6]:
    box('Alley wall poster',(-10.26,1.8,z),(.025,1.1,.65),[PLUM,TEAL,PALE][z%3])
    box('Alley drain',(-13,.025,z),(1.8,.025,.6),BLACK)
    for x in [-13.6,-13.2,-12.8,-12.4]:box('Drain grate',(x,.05,z),(.05,.03,.6),SLATE)
for z in [-35,-31,-27]:beam('External utility conduit',(-20.8,4.7,z),(-20.8,4.7,z+3.7),.1,SLATE)

section('07 Direction signs and light fixtures')
for body,p in [('A  환승 정비고  ↑',(-2,3.0,-29)),('B  시청 광장  →',(18,3.8,-5)),('시청 광장  ↑',(35,3.2,4)),('WELCOME / MONATIUM',(-4,3.8,-42.2))]:
    box('Wayfinding frame',p,(7, .85,.18),BLACK,.08)
    text('Wayfinding',body,(p[0],p[1],p[2]-.11),.4,CYAN)
for b in layout['blocks']:
    if 'step' in b['name']:
        x,y,z=b['center'];w,h,d=b['size']
        box('Step nosing',(x,h+.012,z),(w,.024,min(.07,d)),PALE)
    if b['name']=='Transit freight cover':
        x,y,z=b['center']
        for dx in [-1,1]:box('Cargo corner',(x+dx,y,z-1.02),(.12,1.5,.06),PALE)
        text('Cargo serial','M / 041',(x,y,z-1.04),.24,PALE)
for r in layout['rooms']:
    if 'lounge' in r['name'].lower() or 'sanctuary' in r['name'].lower() or 'dispatch' in r['name'].lower():
        x,z=r['x'],r['z'];d=r['d'];h=r['height']
        box('Spawn ceiling luminaire',(x,h-.1,z),(8,.08,1.4),WHITE)
        for dx in [-5,5]:box('Sanctuary locker',(x+dx,1.2,z),(1.1,2.4,.75),TEAL,.08)
        text('Spawn label','MONATIUM / SAFE ZONE',(x,h-.8,z-d/2-.28),.4,CYAN)

section('08 Stars and night lighting')
for i in range(140):
    a=random.uniform(0,math.tau);e=random.uniform(.35,1.45);r=135
    s=random.uniform(.035,.095)
    sphere('Distant star',(23+r*math.cos(e)*math.cos(a),r*math.sin(e),-8+r*math.cos(e)*math.sin(a)),(s,s,s),WHITE)
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=24
scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.045,.075,.15,1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value=.7
bpy.ops.object.light_add(type='SUN');sun=bpy.context.object;sun.name='Blue hour moon'
sun.rotation_euler=(.45,-.6,-.4);sun.data.energy=1.6;sun.data.color=(.38,.57,1);sun.data.angle=.2
lights=[(-2,6,-34,[108,173,255],700,10),(-15,4.6,-28,[80,255,220],550,9),
        (6,8,-5,[96,213,255],1400,14),(45,7,20,[255,202,120],1800,14),
        (25,4.5,17,[190,130,255],450,8),(35,6,8,[80,220,220],800,10)]
for x,y,z,c,power,radius in lights:
    bpy.ops.object.light_add(type='AREA',location=loc((x,y,z)))
    o=bpy.context.object;o.name='District fill';o.data.energy=power;o.data.color=tuple(v/255 for v in c);o.data.shape='DISK';o.data.size=radius
scene.view_settings.view_transform='AgX'
scene.render.resolution_x=1440;scene.render.resolution_y=900;scene.render.resolution_percentage=100
def camera(name,p,target,lens=24,ortho=None):
    bpy.ops.object.camera_add(location=loc(p));o=bpy.context.object;o.name=name
    o.rotation_euler=(Vector(loc(target))-o.location).to_track_quat('-Z','Y').to_euler();o.data.lens=lens;o.data.clip_end=500
    if ortho:o.data.type='ORTHO';o.data.ortho_scale=ortho
    return o
cams=[camera('Civic preview',(37,7.5,35),(55,6,18),22),camera('Arrival',(-3,2,-40),(5,6,-15)),
      camera('Transit hall',(11,2,-15),(5,4,5)),camera('Mint PC',(-15,2,-35),(-15,2,-23),20),
      camera('Studio',(25,2,12),(26,1.5,20)),camera('Layout',(23,125,-8),(23,0,-8),ortho=145)]
scene.camera=cams[0]
# Stable mesh QA and UVs for Fyrox tangent generation, including text geometry.
mesh_objects=[o for o in scene.objects if o.type=='MESH']
for o in mesh_objects:
    mesh=o.data;bm=bmesh.new();bm.from_mesh(mesh)
    bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.000001)
    bmesh.ops.dissolve_degenerate(bm,edges=list(bm.edges),dist=.000001)
    bmesh.ops.triangulate(bm,faces=list(bm.faces));bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
    bad=[f for f in bm.faces if f.calc_area()<1e-10]
    if bad:bmesh.ops.delete(bm,geom=bad,context='FACES')
    bm.to_mesh(mesh);bm.free();mesh.update()
    # Preserve the original illustration's normalized UVs.
    if o.name.startswith('Elena banner'):
        continue
    uv=mesh.uv_layers.new(name='Planar UV')
    for face in mesh.polygons:
        axis=max(range(3),key=lambda i:abs(face.normal[i]));axes=[i for i in range(3) if i!=axis]
        for li in face.loop_indices:
            co=mesh.vertices[mesh.loops[li].vertex_index].co;uv.data[li].uv=(co[axes[0]]*.25,co[axes[1]]*.25)
stats=dict(slug='elf-city',objects=len(mesh_objects),vertices=sum(len(o.data.vertices) for o in mesh_objects),
    triangles=sum(len(o.data.polygons) for o in mesh_objects),collision_blocks=len(layout['blocks']),stairs=len(layout['stairs']),materials=sum('srgb' in m for m in bpy.data.materials))
scene['map_slug']='elf-city';scene['layout_source']='layout.json'
scene['coordinates']='Blender (x,-game_z,game_y); FBX import scale 0.01'
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'elf-city.blend'))
# Batch spatially by material, retaining the editable original above.
groups={}
for o in mesh_objects:
    center=sum((o.matrix_world@Vector(c) for c in o.bound_box),Vector())/8
    key=(math.floor(center.x/20),math.floor(-center.y/20),o.data.materials[0].name)
    groups.setdefault(key,[]).append(o)
exports=[]
for key,objects in groups.items():
    bpy.ops.object.select_all(action='DESELECT')
    for o in objects:o.select_set(True)
    bpy.context.view_layer.objects.active=objects[0]
    if len(objects)>1:bpy.ops.object.join()
    o=bpy.context.object;o.name='ElfCity_%s_%s_%s'%key;exports.append(o)
for m in bpy.data.materials:
    if 'srgb' in m:
        rgba=tuple(v/255 for v in m['srgb'])+(1,)
        m.diffuse_color=rgba;m.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value=rgba
bpy.ops.object.select_all(action='DESELECT')
for o in exports:o.select_set(True)
for image in bpy.data.images:
    if image.name == 'elena-banner.png':
        image.filepath = str(OUT/'elena-banner.png')
bpy.ops.export_scene.fbx(filepath=str(OUT/'elf-city.fbx'),use_selection=True,object_types={'MESH'},
    apply_unit_scale=True,axis_forward='-Z',axis_up='Y',bake_anim=False,path_mode='AUTO')
stats['export_chunks']=len(exports)
(OUT/'build-report.json').write_text(json.dumps(stats,indent=2)+'\n',encoding='utf-8')
img=bpy.data.images.new('Emission white',width=2,height=2);img.pixels=[1.0]*16
img.filepath_raw=str(OUT/'emission.png');img.file_format='PNG';img.save()
print('ELF_MODEL_OK',json.dumps(stats),flush=True)
if '--render' in sys.argv:
    bpy.ops.wm.open_mainfile(filepath=str(OUT/'elf-city.blend'));scene=bpy.context.scene
    names=['Civic preview','Arrival','Transit hall','Mint PC','Studio','Layout']
    if '--preview-only' in sys.argv:names=names[:1]
    for name in names:
        scene.camera=bpy.data.objects[name]
        scene.render.filepath=str(OUT/'preview.png' if name=='Civic preview' else ART/(name.lower().replace(' ','-')+'.png'))
        bpy.ops.render.render(write_still=True)
    print('ELF_RENDER_OK',flush=True)
