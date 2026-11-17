"""Metres, game X/Y/Z. Shared collision source for Blender and the server.

Gibraltar-inspired circulation: underpass/overpass, vehicle hall with catwalks,
then a bent street opening into a civic plaza. Adapted to TSB's two objectives.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/maps/elf-city'
BLUE, SLATE, TEAL, PALE = [55, 70, 98], [80, 102, 125], [39, 126, 135], [157, 178, 187]
blocks, stairs, rooms = [], [], []

def block(name, p, size, color=BLUE, phaseable=True):
    assert min(size) > 0
    blocks.append(dict(name=name, center=p, size=size, color=color, phaseable=phaseable))

def wall(name, axis, fixed, lo, hi, doors=(), height=6, bottom=0,door_height=3.2):
    def part(a, b, y, h, suffix=''):
        if b <= a: return
        p = [(a+b)/2, y+h/2, fixed] if axis == 'x' else [fixed, y+h/2, (a+b)/2]
        size = [b-a, h, .45] if axis == 'x' else [.45, h, b-a]
        block(name+suffix, p, size)
    cursor = lo
    for a, b in sorted(doors):
        part(cursor, a, bottom, height)
        part(a, b, bottom+door_height, max(.3,height-door_height), ' lintel')
        cursor = b
    part(cursor, hi, bottom, height)

def stair(name, x, z, dx, dz, count=12, width=3.1, base=0):
    for n in range(count):
        h = base + (n+1)*.3
        block(name+' step %02d'%n, [x+dx*n,h/2,z+dz*n],
              [abs(dx) if dx else width,h,abs(dz) if dz else width],SLATE)
    stairs.append(dict(name=name,start=[x-dx*1.2,base,z-dz*1.2],
                       end=[x+dx*(count-.3),base+count*.3,z+dz*(count-.3)]))

def room(name,x,z,w,d,height=5,front=True,back=True,side=None,roof=True):
    wall(name+' front','x',z-d/2,x-w/2,x+w/2,[(x-2,x+2)] if front else [],height)
    wall(name+' rear','x',z+d/2,x-w/2,x+w/2,[(x-2,x+2)] if back else [],height)
    for sign in [-1,1]:
        wall(name+' side','z',x+sign*w/2,z-d/2,z+d/2,
             [(z-2,z+2)] if side == sign else [],height)
    if roof: block(name+' roof',[x,height+.15,z],[w+.5,.3,d+.5],SLATE)
    rooms.append(dict(name=name,x=x,z=z,w=w,d=d,height=height))

def make_layout():
    blocks.clear(); stairs.clear(); rooms.clear()
    for x in [-22.5,68.5]: block('Boundary wall',[x,7,-8],[1,14,94],BLUE,False)
    for z in [-54.5,38.5]: block('Boundary wall',[23,7,z],[92,14,1],BLUE,False)
    # Buildings keep the opening street separate from the final civic district.
    block('Sealed research megablock',[44,12,-32],[47,24,44],BLUE,False)
    block('Northern utility campus',[4,7,32],[50,14,12],BLUE,False)
    room('Arrival lounge',-4,-47,15,9,5.5,False,True)
    room('Forward network lounge',4,20,15,9,5,True,False)
    room('Security dispatch',36,-4,15,9,5,True,True)
    room('City hall sanctuary',58,31,16,10,6,False,False,-1)
    # A side cafe joins the main underpass before the hangar door.
    room('Mint PC cafe',-15,-28,11,18,6,True,True,1)
    # Two-height cafe openings join its mezzanine with the street overpass.
    blocks[:]=[b for b in blocks if not (b['name'].startswith('Mint PC cafe rear') or
        (b['name'].startswith('Mint PC cafe side') and b['center'][0]>-15))]
    wall('Mint PC cafe rear','x',-19,-20.5,-9.5,[(-18.8,-13)],6,door_height=5.7)
    wall('Mint PC cafe side','z',-9.5,-37,-19,[(-30,-24)],6,door_height=5.7)
    block('East service kiosk',[12,3.2,-32],[6,6.4,10])
    block('Service balcony',[14,3.45,-26],[10,.3,3],SLATE)
    stair('Arrival overpass stairs',18,-40,0,.9)
    block('Overpass east landing',[18,3.45,-27.075],[3,.3,5.15],SLATE)
    block('Street pedestrian overpass',[1.5,3.45,-26],[35,.3,3.2],SLATE)
    for x in [-8,10]: block('Overpass pier',[x,1.65,-26],[.6,3.3,.9],SLATE)
    block('Overpass west landing',[-16.8,3.45,-22],[3.2,.3,8],SLATE)
    stair('West overpass stairs',-16.8,-9,0,-.9)
    for x in [-3,3,9]: block('Street road divider',[x,.55,-35],[2,1.1,.9],PALE)
    # Tall hall: two ground doors, an open west service lane, roof lanterns.
    wall('Transit hangar entry','x',-17,-10,22,[(-5,4),(12,16)],9)
    wall('Transit hangar west','z',-10,-17,10,[(-10,-6),(3,7)],9)
    wall('Transit hangar east','z',22,-17,10,[(-6,2)],9)
    wall('Transit hangar rear','x',10,-10,22,[(-2,3),(14,19)],9)
    block('Hangar west roof strip',[-7,9.2,-3.5],[6,.4,27],SLATE)
    block('Hangar east roof strip',[19,9.2,-3.5],[6,.4,27],SLATE)
    # Docked monorail replaces the shuttle; walkable roof has two approaches.
    block('Docked train hull',[6,1.8,-5],[4.6,3.6,13],PALE)
    block('Train maintenance deck',[5.8,3.45,3],[7,.3,3.2],SLATE)
    block('Hangar west catwalk',[-6.5,3.45,1.3],[3,.3,13.4],SLATE)
    block('Catwalk train bridge',[-.8,3.45,3],[10,.3,2.8],SLATE)
    stair('Hangar catwalk stairs',-6.5,-14.2,0,.8)
    stair('Train deck stairs',17,3,-.7,0)
    for x,z in [(-6.5,-2),(-6.5,6),(2,3)]:
        block('Catwalk column',[x,1.65,z],[.5,3.3,.5],SLATE)
    for x,z in [(15,-12),(16,7),(-3,-5)]:
        block('Transit freight cover',[x,.8,z],[2.5,1.6,2],TEAL)
    # The exit turns twice. The internal workstation is a genuine flank route.
    room('Night shift studio',25.5,16.5,9,13,6,True,True,1)
    block('Promenade divider',[35,.65,12],[8,1.3,2],PALE)
    block('Cafe arcade wall',[49,4,3],[19,8,5],BLUE)
    block('Cafe arcade canopy',[49,4.6,7],[20,.35,3],SLATE)
    for x in [40,49,58]: block('Cafe arcade column',[x,2.2,8],[.55,4.4,.55],SLATE)
    # B plaza has unequal entrances and an open ground route beside city hall.
    block('City hall facade',[61,6.5,18],[10,13,16],BLUE)
    for x,z,w,d in [(36,24,3,5),(45,29,5,2.5),(43,14,3,2.5),(30,7,2.5,2.5)]:
        block('Planted concrete island',[x,.6,z],[w,1.2,d],SLATE)
    for x,z in [(-19,-39),(17,-20),(30,32)]:
        block('Street tree planter',[x,.55,z],[2.8,1.1,2.8],SLATE)

    # Furniture colliders are skinned by detailed Blender models, not box meshes.
    def prop(name,p,size):
        block(name,p,size)
        blocks[-1]['render']=False
    for z in [-33,-29,-25]:
        for x in [-18,-12]:
            prop('PC desk',[x,.48,z],[1.4,.96,1.05])
            prop('PC screen',[x,1.55,z+.28],[1.18,.8,.15])
            prop('PC chair',[x,.8,z-1.15],[.7,1.3,.6])
    for x in [23,25]:
        prop('Studio desk',[x,.48,20],[1.4,.96,1.05])
        prop('Studio chair',[x,.8,18.85],[.7,1.3,.6])
    prop('Studio bed',[28,.6,20],[2.2,1.2,3.6])
    for x in [22,23]:prop('Studio server rack',[x,1.4,12],[.75,2.8,1])
    prop('Viaduct plaza pier',[36,6,26],[1.3,12,1.8])
    for x,z in [(51,6.3),(-17,-36),(29,5)]:prop('Vending machine',[x,1.25,z],[1.4,2.5,.85])

    return dict(slug='elf-city',name='ELF CITY',display_name='엘프 도시',
        bounds=[-22,68,-54,38],points=[[12,0,-2],[46,0,21]],
        spawns=dict(attack_a=[-4,0,-47],attack_b=[4,0,20],defense_a=[36,0,-4],defense_b=[58,0,31]),
        blocks=blocks,stairs=stairs,rooms=rooms,
        high_ground=[[0,3.6,-26],[-6.5,3.6,2],[6,3.6,-5]],
        research='../../references/elf-city/sources.json')

if __name__ == '__main__':
    OUT.mkdir(parents=True,exist_ok=True)
    data=make_layout()
    (OUT/'layout.json').write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('ELF_LAYOUT_OK',len(blocks),'solids',len(stairs),'stairs')
