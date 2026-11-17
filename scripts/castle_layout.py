"""Anubis-derived topology; metres. +Z attacker street turns into +X temple.
Shared by Blender, collision and navigation; distances adapted to TSB.
"""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/maps/fairy-kingdom-castle'
IVORY,STONE,TEAL,GOLD=[231,235,220],[158,189,184],[76,167,158],[227,184,88]
blocks,stairs,roofs,gates=[],[],[],[]

def block(name,center,size,color=IVORY,phaseable=None):
    assert min(size)>0,(name,size)
    blocks.append(dict(name=name,center=center,size=size,color=color,
        phaseable=size[1]>.32 if phaseable is None else phaseable))

def segment(name,axis,fixed,lo,hi,height,bottom=0):
    p=[(lo+hi)/2,bottom+height/2,fixed] if axis=='x' else [fixed,bottom+height/2,(lo+hi)/2]
    size=[hi-lo,height,.4] if axis=='x' else [.4,height,hi-lo]
    block(name,p,size)

def wall(name,axis,fixed,lo,hi,openings=(),height=6,bottom=0):
    cursor=lo
    for a,b in sorted(openings):
        if a>cursor: segment(name,axis,fixed,cursor,a,height,bottom)
        segment(name+' lintel',axis,fixed,a,b,max(.3,height-3),bottom+3)
        cursor=b
    if hi>cursor: segment(name,axis,fixed,cursor,hi,height,bottom)

def stair(name,x,z,dx,dz,count=12,width=2.6):
    for n in range(count):
        h=(n+1)*.3
        block(name+' step %02d'%n,[x+dx*n,h/2,z+dz*n],
            [abs(dx) if dx else width,h,abs(dz) if dz else width],STONE)
    stairs.append(dict(name=name,start=[x-dx*1.2,0,z-dz*1.2],
        end=[x+dx*(count-.3),count*.3,z+dz*(count-.3)],height=count*.3))

def roof(name,x,z,w,d,y=7):
    block(name+' roof',[x,y-.15,z],[w,.3,d],TEAL)
    roofs.append(dict(name=name,x=x,z=z,w=w,d=d,y=y))

def spawn_room(name,x,z):
    for side in [-1,1]:
        block(name+' side',[x+side*7.15,2.5,z],[.4,5,8.4])
        wall(name+' exit','x',z+side*4.2,x-7.15,x+7.15,[(x-2.3,x+2.3)],5)
        block(name+' bench',[x+side*5.5,.5,z],[1,.22,2.5],TEAL)
    roof(name,x,z,14.7,8.8,5.25)

def west_gallery():
    # Unique A west room. The opposite side is a solid keep, not its mirror.
    x,z=-12,12
    block('A west gallery slab',[x+.7,3.45,z],[8,.3,11.2])
    roof('A west gallery',x,z,6.8,11.8)
    block('A west gallery outer wall',[x-3,3.4,z],[.3,6.8,11.2])
    for end in [-5.6,5.6]:
        for base in [0,3.6]: wall('A west gallery door','x',z+end,x-3,x+3,[(x-1.2,x+1.2)],3.2,base)
    for offset in [-3.45,3.45]: block('A west arcade wall',[x+3,1.65,z+offset],[.3,3.3,4.3])
    block('A west arcade lintel',[x+3,2.95,z],[.3,.7,2.6])
    for offset,width in [(-5.1,1),(-1.95,1.3),(1.95,1.3),(5.1,1)]:
        block('A west window pier',[x+3,5.2,z+offset],[.3,3.2,width])
    for offset in [-3.6,3.6]: block('A west window sill',[x+3,3.925,z+offset],[.38,.65,2])
    for offset,length in [(-3.55,3.9),(2.5,1.8)]:
        block('A west parapet',[x+4.6,3.925,z+offset],[.2,.65,length],STONE)
    stair('A west stair',x,z-17.1,0,1)

def make_layout():
    for seq in [blocks,stairs,roofs,gates]: seq.clear()
    for x in [-22.5,68.5]: block('Outer boundary',[x,6,-8],[1,12,94],STONE,False)
    for z in [-54.5,38.5]: block('Outer boundary',[23,6,z],[92,12,1],STONE,False)
    # The playable footprint bends: the inaccessible cliff prevents shortcuts
    # between the initial street and B, including for phasing characters.
    block('Inaccessible garden escarpment',[46.5,7,-29.5],[43,14,49],STONE,False)
    block('West rear garden boundary',[-13,6,35],[18,12,6],STONE,False)
    spawn_room('Attack garden lodge',5,-45)
    spawn_room('Forward garden sanctuary',-13,26)
    spawn_room('Royal defense sanctuary',58,30)
    # Broad approach with unequal buildings and a two-way first choke.
    wall('Approach west house front','x',-29,-18,-6,[(-14,-10)],6)
    wall('Approach west house rear','x',-9,-18,-6,[(-14,-10)],6)
    wall('Approach west house west','z',-18,-29,-9,[],6)
    wall('Approach west house east','z',-6,-29,-9,[(-18,-14)],6)
    roof('Approach west house',-12,-19,12.4,20.4,6.3)
    block('Approach east gatehouse',[16,4,-20],[12,8,15])
    roof('Approach east gatehouse',16,-20,12.4,15.4,8.2)
    for x,z in [(-2,-32),(9,-28)]: block('Garden herald plinth',[x,.8,z],[2.2,1.6,2.2],STONE)
    wall('Outer gate wall','x',-8,-22,25,[(-14.2,-9.8),(-2,6)],6.2)
    gates.append(dict(name='Outer crown gate',x=2,z=-8,axis='z',opening=8,height=6.2))
    west_gallery()
    # Off-axis A court and rounded projecting defense platform.
    for z,w in [(.5,5),(1.5,7),(2.5,8),(4.5,8)]:
        block('A crescent high ground',[12,3.45,z],[w,.3,1 if z<4 else 3],STONE)
    for x in [9,15]:
        for z in [1.5,5.2]:
            block('A crescent support',[x,1.65,z],[.75,3.3,.75])
            block('A crescent upper column',[x,5.1,z],[.75,3,.75])
    stair('A crescent stair',23.5,4,-.65,0)
    block('A east keep',[14,5.8,14],[7,11.6,9])
    block('A keep west balcony',[9,3.45,14],[3,.3,9])
    block('A keep rear balcony',[14,3.45,20],[13,.3,3])
    block('A keep east balcony',[19,3.45,14],[3,.3,9])
    block('A keep front balcony',[14,3.45,8.25],[13,.3,2.5])
    block('A balcony link',[12,3.45,6.5],[4,.3,1])
    roof('A royal keep',14,14,7.5,9.5,11.9)
    for z in [10,18]: block('A keep arcade support',[8.2,1.65,z],[.65,3.3,.65])
    # The keep hides B from A. Two unequal streets bend around the mass.
    block('Connector wayside pavilion',[23,3.3,13],[4,6.6,10])
    roof('Connector pavilion',23,13,4.5,10.5,6.9)
    # B faces west (+X attack), perpendicular to the initial attacker street.
    wall('B west entrance','z',28,5,25,[(12,18)],7.2)
    gates.append(dict(name='Inner crown gate',x=28,z=15,axis='x',opening=6,height=7.2))
    for name,z,first in [('B north inner wall',25,(31,35)),('B south inner wall',5,(29,33))]:
        wall(name,'x',z,28,38.3,[first],7.2)
        wall(name,'x',z,41.7,54,[(46,50) if z==25 else (49,53)],7.2)
        segment(name+' closed upper doorway','x',z,38.3,41.7,7.2)
    wall('B throne wall','z',54,5,25,[(7,10.5),(20,23.5)],9)
    block('B crown dais',[52.2,.6,15],[2.8,1.2,4.2],GOLD)
    block('B crown throne back',[53,3,15],[1,4.8,3],IVORY)
    # Wide north rooms versus narrow south dogleg, not mirrored side lanes.
    wall('B north room outside','x',35,24,52,[],8)
    wall('B north room entry','z',24,25,35,[(28,32)],7.2)
    wall('B north room east','z',52,25,35,[],7.2)
    wall('B north room baffle','z',38,25,35,[(30,34)],5.5)
    roof('B north library',38,30,28.4,10.4,8.3)
    wall('B south passage outside','x',-4,24,54,[],7)
    wall('B south passage entry','z',24,-4,5,[(-1,4.8)],7)
    wall('B south passage turn','z',46,-4,5,[(-3,0)],6)
    wall('B south passage end','z',54,-4,5,[],7)
    roof('B south passage',39,.5,30.4,9.4,7.3)
    for x in [32,37,42]:
        for z in [7.5,22.5]: block('B temple column',[x,3,z],[1.2,6,1.2])
    for x in [35,45,50]:
        for z in [10,20]: block('B temple column',[x,3,z],[1.1,6,1.1])
    for x,z in [(-3,22),(21,-2),(59,16)]: block('Royal planter',[x,.75,z],[2.6,1.5,2.6],STONE)
    points=[[0,0,12],[46,0,15]]
    spawns=dict(attack_a=[5,0,-45],attack_b=[-13,0,26],defense_a=[58,0,30],defense_b=[58,0,30])
    routes=[
        dict(name='A main choke',kind='main',points=[[5,0,-45],[5,0,-34],[2,0,-30],[2,0,-8],[2,0,-1],[0,0,12]]),
        dict(name='A west house flank',kind='flank',points=[[5,0,-34],[-12,0,-34],[-12,0,-29],[-12,0,-9],[-12,0,-6.6],[-19,0,-6.6],[-19,0,20],[-3,0,20],[0,0,12]]),
        dict(name='A west upper room',kind='high',points=[stairs[0]['start'],stairs[0]['end'],[-12,3.6,12],[-8,3.6,12],[-5,0,12],[0,0,12]]),
        dict(name='A crescent and keep',kind='high',points=[stairs[1]['start'],stairs[1]['end'],[12,3.6,4],[12,3.6,8],[9,3.6,8],[9,3.6,14],[9,3.6,20]]),
        dict(name='A to B turning street',kind='main',points=[[0,0,12],[4,0,23],[22,0,23],[26,0,21],[26,0,15],[30,0,15],[46,0,15]]),
        dict(name='B north library flank',kind='flank',points=[[22,0,23],[22,0,30],[26,0,30],[34,0,32],[42,0,32],[48,0,28],[48,0,24],[46,0,15]]),
        dict(name='B south bent corridor',kind='flank',points=[[2,0,-1],[19,0,0],[26,0,0],[42,0,0],[44,0,-1.5],[48,0,-1.5],[51,0,2],[51,0,7],[52,0,9],[52,0,12],[46,0,12],[46,0,15]]),
    ]
    layout = dict(schema=2,revision='anubis-topology-v2',slug='fairy-kingdom-castle',name='FAIRY KINGDOM CASTLE',
        name_ko='요정왕국 성',bounds=[-22,68,-54,38],upper_floor=3.6,points=points,spawns=spawns,spawn_half_size=[6.5,2.25,3],
        blocks=blocks,routes=routes,stairs=stairs,roofs=roofs,gates=gates,
        footprint=[[-22,-54],[25,-54],[25,-5],[68,-5],[68,38],[-4,38],[-4,32],[-22,32]],
        reference=dict(plan='../../references/fairy-kingdom-castle/anubis-layout.jpg',
            attribution='Joe Iz Gaming plan; Orhin_InsaLan A/B 3D diagrams',
            method='Rotated topology reconstruction; TSB-adjusted dimensions, not a metric scan.'))
    from castle_concept_layout import enhance
    from castle_gate_arch import configure_outer_gate
    layout = enhance(layout)
    configure_outer_gate(layout)
    return layout

if __name__=='__main__':
    OUT.mkdir(parents=True,exist_ok=True)
    layout=make_layout()
    (OUT/'layout.json').write_text(json.dumps(layout,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('CASTLE_LAYOUT_OK',len(blocks),'collision blocks;',len(stairs),'distinct stairs')
