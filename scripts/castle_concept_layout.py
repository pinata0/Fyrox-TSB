"""Place the nine user concept spaces on the retained Anubis circulation.
Furniture proxies are shared with the server, but rendered by detailed assets.
"""
import math

def enhance(layout):
    blocks=layout['blocks'];props=[];lights=[]
    def solid(name,p,size,color=(228,232,224),render=True,phaseable=True):
        b=dict(name=name,center=p,size=size,color=list(color),phaseable=phaseable if size[1]>.32 else False,render=render)
        blocks.append(b);return b
    def remove(*names): blocks[:]=[b for b in blocks if b['name'] not in names]
    def wall(name,axis,fixed,lo,hi,doors=(),height=8):
        cursor=lo
        def part(a,b,y,h):
            p=[(a+b)/2,y+h/2,fixed] if axis=='x' else [fixed,y+h/2,(a+b)/2]
            s=[b-a,h,.4] if axis=='x' else [.4,h,b-a]
            solid(name,p,s)
        for a,b in doors:
            if a>cursor:part(cursor,a,0,height)
            part(a,b,3,height-3);cursor=b
        if cursor<hi:part(cursor,hi,0,height)
    def prop(kind,p,reference,space,turn=0,variant='',proxy=None):
        idx=len(props)
        item=dict(id=f'{space}-{kind}-{idx:03}',kind=kind,position=p,turn=turn,reference=reference,space=space,variant=variant)
        props.append(item)
        if proxy:
            offset,size=proxy;c,s=math.cos(turn),math.sin(turn)
            center=[p[0]+offset[0]*c+offset[2]*s,p[1]+offset[1],p[2]-offset[0]*s+offset[2]*c]
            solid('Furniture '+item['id'],center,[abs(c)*size[0]+abs(s)*size[2],size[1],abs(s)*size[0]+abs(c)*size[2]],render=False)
        return item
    def light(p,color,radius,intensity):lights.append(dict(position=p,color=color,radius=radius,intensity=intensity))

    # 02: bedroom inside the initial sanctuary. Furniture is kept behind the
    # six horizontal spawn slots; both doors and the full slot row stay clear.
    remove('Attack garden lodge bench','Attack garden lodge side')
    for x in [-2.15,12.15]:
        for z,d in [(-48.1,2.2),(-41.9,2.2)]:solid('Bedroom window pier',[x,2.5,z],[.4,5,d])
        solid('Bedroom window sill',[x,.6,-45],[.4,1.2,4])
        solid('Bedroom window header',[x,4.85,-45],[.4,.3,4])
        solid('Bedroom glass barrier',[x,2.9,-45],[.12,3.4,4],render=False)
    prop('bed',[1,0,-47.3],[2],'bedroom',variant='mint',proxy=([0,.8,0],[2.2,1.6,3.2]))
    prop('bedside',[3.3,0,-47.4],[2],'bedroom',proxy=([0,.65,0],[1,1.3,1]))
    prop('flowerpot',[11.85,1.2,-46.4],[2],'bedroom')
    for x,turn in [(-1.92,-math.pi/2),(11.92,math.pi/2)]:
        prop('arched_window',[x,1.2,-45],[2],'bedroom',turn,variant='curtain')
    prop('oval_rug',[6,.035,-46.3],[2],'bedroom',variant='mint')
    prop('wall_frame',[1.2,2.7,-48.96],[2],'bedroom',math.pi,variant='tree')
    prop('wall_frame',[8.5,2.8,-48.96],[2],'bedroom',math.pi,variant='crown')
    light([7.5,3.7,-45],[255,239,194],10,2.0)

    # 04/06: a real cafe interior and a three-step outdoor terrace replace the
    # formerly solid east gatehouse. The main approach remains at X=2.
    remove('Approach east gatehouse')
    wall('Cafe south frontage','x',-27.5,10,22,[(14,18)],5.3)
    wall('Cafe north wall','x',-12.5,10,22,[],5.3)
    wall('Cafe west doorway','z',10,-27.5,-12.5,[(-22,-18)],5.3)
    wall('Cafe east wall','z',22,-27.5,-12.5,[],5.3)
    for b in blocks:
        if b['name']=='Approach east gatehouse roof':b['center'][1]=5.45
    solid('Cafe raised interior',[16,.3,-20],[12,.6,15],color=(152,110,69))
    solid('Cafe terrace',[7.75,.3,-20],[4.5,.6,11],color=(152,110,69))
    for i in range(3):solid('Cafe terrace step',[4.25+i*.5,(i+1)*.1,-20],[.5,(i+1)*.2,4.4],color=(185,147,96))
    for i in range(2):solid('Cafe front step',[16,(i+1)*.1,-28.25+i*.5],[4,(i+1)*.2,.5],color=(185,147,96))
    prop('cafe_counter',[16,.6,-14],[6],'cafe',proxy=([0,.65,0],[7,1.3,1.3]))
    for x,z,y in [(7.7,-24,.6),(7.7,-16,.6),(14,-24,.6),(19,-18,.6)]:
        prop('cafe_table',[x,y,z],[6],'cafe',proxy=([0,.65,0],[1.5,1.3,1.5]))
        for dz,t in [(-1.25,math.pi),(1.25,0)]:
            prop('chair',[x,y,z+dz],[6],'cafe',t,proxy=([0,.7,0],[.75,1.4,.75]))
    prop('menu_board',[3.9,0,-26.5],[6],'cafe',-.2,proxy=([0,.85,0],[.85,1.7,.6]))
    for x,z in [(-5.3,-25),(-5.3,-11),(8,-29),(23,-25),(23,-14)]:
        prop('flowerpot',[x,0,z],[4,6],'street',proxy=([0,.5,0],[.8,1,.8]))
    light([16,4.5,-20],[255,222,172],12,2.1)
    for z in [-25.5,-14.5]:solid('Cafe rail proxy',[7.75,1.1,z],[4.5,1,.18],render=False)
    for lo,hi in [(-25.5,-22.3),(-17.7,-14.5)]:solid('Cafe edge rail proxy',[5.55,1.1,(lo+hi)/2],[.18,1,hi-lo],render=False)

    # 05: study in the west through-house, retaining the central flank passage.
    wall('Study arched partition','x',-19,-18,-6,[(-14,-10)],5.8)
    prop('bed',[-8,0,-12.7],[5],'study',variant='rose',proxy=([0,.8,0],[2.2,1.6,3.2]))
    for z in [-12,-16]:prop('bookshelf',[-16.9,0,z],[5],'study',-math.pi/2,proxy=([0,1.7,0],[2.8,3.4,.65]))
    prop('desk',[-15.8,0,-22],[5],'study',-math.pi/2,proxy=([0,.7,0],[3,1.4,1.4]))
    prop('satchel',[-14.4,0,-23.3],[5],'study',proxy=([0,.45,0],[.7,.9,.4]))
    prop('oval_rug',[-11.8,.035,-14.8],[5],'study',variant='green')
    prop('study_arch',[-12,0,-18.74],[5],'study')
    prop('pinboard',[-17.72,2.7,-23.5],[5],'study',-math.pi/2)
    light([-11,3.8,-14],[255,233,183],10,1.9)
    light([-11,3.8,-25],[245,225,191],9,1.3)

    # 07: village green off the main road, with a walkable ring around its tree.
    prop('village_tree',[16,0,-35],[7],'square',proxy=([0,4,0],[2.6,8,2.6]))
    prop('clock_plinth',[16,0,-38],[7],'square',proxy=([0,.6,0],[1.4,1.2,1]))
    for x,z in [(11.4,-36.5),(20.6,-36.5)]:prop('street_lamp',[x,0,z],[7],'square',proxy=([0,1.7,0],[.35,3.4,.35]))

    # 03: the east keep now encloses a tree atrium. Offset entrances preserve
    # the A-to-B sightline break, and leave an alternate indoor dogleg.
    remove('A east keep','A royal keep roof')
    wall('Atrium west door','z',10.5,9.5,18.5,[(10.2,12.8)],11.6)
    wall('Atrium east door','z',17.5,9.5,18.5,[(15.8,18.1)],11.6)
    wall('Atrium front wall','x',9.5,10.5,17.5,[],11.6)
    wall('Atrium rear wall','x',18.5,10.5,17.5,[],11.6)
    prop('atrium_tree',[14,0,14],[3],'atrium',proxy=([0,3.1,0],[1.6,6.2,1.6]))
    for x,z,r in [(16,-35,4.2),(14,14,1.6)]:
        for i in range(12):
            dz=(i+.5)*r/6-r;w=2*math.sqrt(max(0,r*r-dz*dz))
            solid('Circular planter curb',[x,.11,z+dz],[w,.22,r/6],render=False)
    light([14,9.7,14],[244,255,210],12,2.3)

    # 08: the north flank room becomes the guardian hall. Its angled route
    # through the existing partition stays open.
    for x,z,turn in [(25.1,26.8,-math.pi/2),(25.1,33.5,-math.pi/2),(36.8,26.8,math.pi/2),(36.8,33.5,math.pi/2)]:
        prop('guardian',[x,0,z],[8],'guardian_hall',turn,proxy=([0,1.3,0],[1.25,2.6,.8]))
    prop('altar',[36.6,0,28.1],[8],'guardian_hall',math.pi/2,proxy=([0,.7,0],[1.7,1.4,.9]))
    light([30,5.6,29.5],[226,244,255],12,2.2)
    for x in [42,48]:prop('bookshelf',[x,0,34.45],[5],'north_library',proxy=([0,1.7,0],[2.8,3.4,.65]))
    light([45,4,30],[255,234,195],10,1.7)

    # 01: the south corridor gets rib vaults, sconces, and clerestory windows.
    for x in [27,33,39,45,51]:prop('sconce',[x,2.7,-3.72],[1],'corridor',math.pi)
    light([30,5.2,-.8],[234,245,255],9,1.5)
    light([49,5.2,1],[234,245,255],9,1.5)

    # 09: throne and climbable dais behind B, within the old throne footprint.
    remove('B crown dais','B crown throne back')
    for i in range(4):solid('Throne dais step',[49.9+i*.6,.15*(i+1),15],[.6,.3*(i+1),4.2],render=False)
    solid('Throne dais landing',[52.6,.6,15],[1.2,1.2,4.2],render=False)
    prop('throne',[52.6,1.2,15],[9],'throne',math.pi/2,proxy=([0,1.2,0],[1.9,2.4,1.2]))
    prop('royal_side_table',[52.5,1.2,16.5],[9],'throne',proxy=([0,.6,0],[.85,1.2,.85]))
    for z in [10.5,19.5]:prop('guardian',[52.9,0,z],[8,9],'throne',math.pi/2,proxy=([0,1.3,0],[1.4,2.6,.9]))
    for b in blocks:
        if b['name']=='B temple column':b['center'][1]=5;b['size'][1]=10;b['render']=False
    # The royal hall remains open above the ground-level circulation.
    solid('Royal hall ceiling',[41,11.9,15],[26,.3,20],color=(223,228,229))
    for z in [5,25]:solid('Royal hall upper wall',[41,9.55,z],[26,4.7,.4],color=(223,228,229))
    solid('Royal hall east upper wall',[54,10.45,15],[.4,2.9,20],color=(223,228,229))
    light([44,8.2,15],[250,243,221],18,2.1)
    light([51,6.2,15],[255,229,186],12,1.7)
    for x,z,h,r in ([(x,z,10.7,.3) for x in [10.78,17.22] for z in [10,18]]
            +[(x,-3.58,5.4,.24) for x in [26,32,38,44,50,53]]
            +[(x,z,7.9,.32) for x in [24.7,31,37.4] for z in [25.4,34.5]]
            +[(53.35,z,11.2,.37) for z in [6,11,19,24]]):
        solid('Engaged pillar proxy',[x,h/2,z],[r*2,h,r*2],render=False)

    for roof in layout['roofs']:
        if roof['name']=='Approach east gatehouse':roof['style']='cafe';roof['y']=5.6
        elif roof['name'] in ['Approach west house','Attack garden lodge','Forward garden sanctuary']:roof['style']='timber'
        elif roof['name']=='A royal keep':roof['style']='atrium'
        else:roof['style']='palace'
    # Further visit routes are checked by the same actual movement sweep as the
    # original Anubis main/flank/high-ground routes, never just by marker count.
    layout['routes'] += [
        dict(name='Cafe terrace and interior',kind='visit',points=[[2,0,-20],[5.9,.6,-20],[8,.6,-20],[16,.6,-20]]),
        dict(name='Study visit',kind='visit',points=[[-12,0,-26],[-12,0,-14],[-14.2,0,-14]]),
        dict(name='Village green ring',kind='visit',points=[[5,0,-39],[21.5,0,-39],[21.5,0,-31],[7,0,-31]]),
        dict(name='Atrium indoor dogleg',kind='visit',points=[[6,0,11.5],[12,0,11.5],[12,0,16],[16,0,17],[19,0,17]]),
        dict(name='Guardian hall visit',kind='visit',points=[[26,0,30],[30,0,28],[34.5,0,28]]),
        dict(name='Throne dais ascent',kind='visit',points=[[48,0,15],[52.5,1.2,15]])
    ]
    # Dais arrival ends before the actual seat, at its front edge.
    layout['routes'][-1]['points'][-1]=[51.5,1.2,15]
    layout.update(schema=3,revision='concept-spaces-v3',props=props,lights=lights,
        spaces=[dict(id=s,reference=r) for s,r in [('corridor',[1]),('bedroom',[2]),('atrium',[3]),('street',[4]),('study',[5]),('cafe',[6]),('square',[7]),('guardian_hall',[8]),('throne',[9])]])
    return layout
