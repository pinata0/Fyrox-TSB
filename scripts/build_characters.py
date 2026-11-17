"""Author original MVP chibi meshes, a weighted rig and baked clips in Blender.

Run with blender-launcher --background --factory-startup --python this_file.
Reference screenshots / generated guides are inputs for design, never game textures.
All generated scene data is confined to this project. No preferences are modified.
"""
import bpy
import json
import math
import sys
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/characters'
OUT.mkdir(parents=True, exist_ok=True)
SLUGS = ['vivi','naia','erpin','id','aya','picora','chloe','renua','mayo','bella','ui','butter','komi','hilde','shion']
# Palettes transcribed from each saved turnaround, not the old gameplay team colors.
SPECS = [
 ('f4f1f5','852d40','f5f3f0','e96b5f','long'),
 ('bfd8ed','9fd7ec','e7edf6','64dcca','braids'),
 ('f5d990','f6f4e9','c25159','f3a050','long'),
 ('f6f5f7','eeedf2','393743','c886a7','long'),
 ('f5f6fc','f7f6f0','bc7e72','e7ca64','aya_twintails'),
 ('72bec8','ee9bb6','f7f3ef','7de2e5','buns'),
 ('f8e4ad','e6ba62','95608b','ec8362','twintails'),
 ('ededef','34394e','d9b770','d7d2aa','bob'),
 ('807185','82a6c7','f0eee6','8ac7e8','pony'),
 ('e4e4f8','3e4779','b2b7eb','ad8bdf','long'),
 ('a5cda0','f1c35d','f8e7b7','a9db5e','braid'),
 ('dcbf93','f1c668','f8f0df','7ccadd','long'),
 ('f8eeee','f8f4f4','edb3c6','eac05e','long'),
 ('4c4d55','4e5353','f2eddf','c4d56d','pony'),
 ('494e68','444951','42699e','7bcce5','sidepony'),
]
CLIPS = {'Idle':(0,60),'Walk':(63,93),'Run':(96,120),'Jump':(123,153),
         'Attack':(156,180),'Skill':(183,219),'Ultimate':(222,282),
         'Dance':(285,405),'Hurt':(408,420),'Death':(423,453),'Select':(456,528)}

SELECT_GESTURES = ['regal greeting','water wave','queen pose','sleepy stretch','parasol greeting',
                   'candy cheek pose','doll hug','time conductor','collector inspection',
                   'ghost flourish','frog greeting','puppy wave','cat yawn','adjust glasses','dark salute']

def selection_pose(index,t,rot,rig):
    # Each hero gets a distinct authored 2.4s entrance; all settle back to idle.
    rise=min(t/.24,1); fall=min((1-t)/.24,1)
    p=(rise*rise*(3-2*rise))*(fall*fall*(3-2*fall))
    wave=math.sin(t*math.tau*3)*p
    poses=[
        {'UpperArm.R':(-.75,0,.35),'Forearm.R':(-1.15,0,0),'Head':(.12,-.15,0)},
        {'UpperArm.L':(-.35,0,-1.15),'Forearm.L':(-1.2,0,0),'Torso':(.12,0,.08)},
        {'UpperArm.L':(0,0,-.65),'UpperArm.R':(0,0,.65),'Forearm.L':(-.9,0,.2),'Forearm.R':(-.9,0,-.2),'Head':(-.15,0,0)},
        {'UpperArm.L':(-2.1,0,-.35),'UpperArm.R':(-1.9,0,.35),'Head':(-.25,0,.18),'Torso':(-.15,0,0)},
        {'UpperArm.R':(-1.2,-.45,.1),'Forearm.R':(-.8,0,0),'UpperArm.L':(-.4,0,-.25),'Torso':(.12,.22,0)},
        {'UpperArm.L':(-.9,0,-.8),'UpperArm.R':(-.9,0,.8),'Forearm.L':(-1.3,0,0),'Forearm.R':(-1.3,0,0),'Head':(.08,0,.18)},
        {'UpperArm.L':(-.75,0,.35),'UpperArm.R':(-.75,0,-.35),'Forearm.L':(-1.0,0,0),'Forearm.R':(-1.0,0,0),'Torso':(.15,0,0)},
        {'UpperArm.L':(-1.2,0,-.45),'UpperArm.R':(-.6,0,.9),'Forearm.L':(-.3,0,0),'Forearm.R':(-.9,0,0),'Head':(0,.2,0)},
        {'UpperArm.L':(-.9,0,-.2),'Forearm.L':(-1.0,0,0),'UpperArm.R':(-.7,0,-.25),'Head':(.25,-.3,0),'Torso':(0,-.15,0)},
        {'UpperArm.L':(-.25,0,-1.0),'UpperArm.R':(-.25,0,1.0),'Forearm.L':(-.6,0,0),'Forearm.R':(-.6,0,0),'Head':(-.12,0,-.12)},
        {'UpperArm.L':(-.6,0,-1.1),'UpperArm.R':(-.6,0,1.1),'Forearm.L':(-1.1,0,0),'Forearm.R':(-1.1,0,0),'Torso':(.12,0,0)},
        {'UpperArm.R':(-.4,0,1.3),'Forearm.R':(-1.15,0,0),'Thigh.L':(-.3,0,0),'Shin.L':(.5,0,0),'Head':(0,0,.15)},
        {'UpperArm.L':(-1.9,0,-.4),'UpperArm.R':(-.95,0,-.1),'Forearm.R':(-1.4,0,0),'Head':(-.2,0,-.15),'Torso':(-.12,0,0)},
        {'UpperArm.R':(-1.05,0,.2),'Forearm.R':(-1.5,0,0),'Head':(.15,.18,0),'UpperArm.L':(0,0,-.2)},
        {'UpperArm.R':(-1.25,0,.7),'Forearm.R':(-1.4,0,0),'UpperArm.L':(-.4,0,-.15),'Torso':(0,-.25,0),'Head':(-.08,.25,0)},
    ]
    for bone,angles in poses[index].items():rot(bone,*(a*p for a in angles))
    if index in [1,10,11]:rig.pose.bones['Hand.R' if index==11 else 'Hand.L'].rotation_euler.y=.45*wave
    if index==7:rig.pose.bones['Hand.R'].rotation_euler.z=.6*wave
    if index==10:rig.pose.bones['Mount'].location.z=.10*abs(wave)
    if index==9:rig.pose.bones['Torso'].rotation_euler.z=.12*wave
    return .06*math.sin(t*math.pi)*p if index in [0,9,10,11] else .0
MATS = {}
PARTS = []
RIG = None

def mat(hexcolor):
    if hexcolor not in MATS:
        def linear(v): return v/12.92 if v <= .04045 else ((v+.055)/1.055)**2.4
        rgba = tuple(linear(int(hexcolor[i:i+2],16)/255) for i in (0,2,4))+(1,)
        m = bpy.data.materials.new('Color_'+hexcolor)
        m.diffuse_color = rgba
        m.use_nodes = True
        bsdf = m.node_tree.nodes.get('Principled BSDF')
        bsdf.inputs['Base Color'].default_value = rgba
        bsdf.inputs['Roughness'].default_value = .82
        MATS[hexcolor] = m
    return MATS[hexcolor]

def finish(obj,name,color,bone='Head'):
    obj.name = name
    obj.data.materials.clear()
    obj.data.materials.append(mat(color))
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    for p in obj.data.polygons: p.use_smooth=True
    vg=obj.vertex_groups.new(name=bone)
    vg.add(list(range(len(obj.data.vertices))),1,'REPLACE')
    PARTS.append(obj)
    return obj

def ball(name,p,s,c,b='Head',seg=16):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=seg,ring_count=10,location=p)
    obj=bpy.context.object
    obj.scale=s
    return finish(obj,name,c,b)

def box(name,p,s,c,b='Torso',bevel=.035):
    bpy.ops.mesh.primitive_cube_add(size=1,location=p)
    obj=bpy.context.object
    obj.scale=s
    bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
    mod=obj.modifiers.new('Soft toy edges','BEVEL'); mod.width=bevel; mod.segments=2
    bpy.ops.object.modifier_apply(modifier=mod.name)
    return finish(obj,name,c,b)

def tube(name,points,radius,color,bone='Head'):
    curve=bpy.data.curves.new(name,'CURVE'); curve.dimensions='3D'
    curve.resolution_u=8; curve.bevel_depth=radius; curve.bevel_resolution=1
    spline=curve.splines.new('BEZIER'); spline.bezier_points.add(len(points)-1)
    for bp,p in zip(spline.bezier_points,points):
        bp.co=p; bp.handle_left_type='AUTO'; bp.handle_right_type='AUTO'
    obj=bpy.data.objects.new(name,curve); bpy.context.collection.objects.link(obj)
    bpy.ops.object.select_all(action='DESELECT'); obj.select_set(True)
    bpy.context.view_layer.objects.active=obj; bpy.ops.object.convert(target='MESH')
    return finish(bpy.context.object,name,color,bone)

def taper(name,p,r1,r2,depth,c,b='Torso',vertices=20):
    bpy.ops.mesh.primitive_cone_add(vertices=vertices,radius1=r1,radius2=r2,depth=depth,location=p)
    return finish(bpy.context.object,name,c,b)

def ring(name,p,rx,rz,c,b='Head',r=.018):
    return tube(name,[(p[0]+rx*math.sin(i*math.tau/32),p[1],p[2]+rz*math.cos(i*math.tau/32)) for i in range(33)],r,c,b)

def petal(name,a,b,width,color,bone='Head'):
    mid=(Vector(a)+Vector(b))*.5
    obj=ball(name,mid,(width,width*.5,(Vector(b)-Vector(a)).length*.6),color,bone)
    obj.rotation_mode='QUATERNION'; obj.rotation_quaternion=(Vector(b)-Vector(a)).to_track_quat('Z','Y')
    return obj

def cat_ear(sign,color,inner):
    x=sign*.28
    points=[(x-sign*.11,-.04,1.77),(x+sign*.11,-.04,1.77),(x+sign*.06,-.01,2.01),
            (x-sign*.11,.09,1.77),(x+sign*.11,.09,1.77),(x+sign*.06,.07,2.01)]
    mesh=bpy.data.meshes.new('Pointed cat ear');mesh.from_pydata(points,[],[(0,1,2),(5,4,3),(0,3,4,1),(1,4,5,2),(2,5,3,0)])
    obj=bpy.data.objects.new('Pointed cat ear',mesh);bpy.context.collection.objects.link(obj)
    finish(obj,'Pointed cat ear',color)
    for p in obj.data.polygons:p.use_smooth=False
    petal('Pink ear inner',(x,-.055,1.81),(x+sign*.05,-.045,1.96),.052,inner)

def bow(p,c,b='Head',scale=1):
    x,y,z=p
    for side in (-1,1):
        obj=ball('Ribbon loop',(x+side*.085*scale,y,z),(.105*scale,.045*scale,.065*scale),c,b)
        obj.rotation_euler.y=side*.3
    ball('Ribbon knot',p,(.044*scale,.049*scale,.047*scale),c,b)

def flower(p,c,b='Head',scale=1):
    for i in range(5):
        t=i*math.tau/5
        ball('Flower petal',(p[0]+math.sin(t)*.064*scale,p[1],p[2]+math.cos(t)*.064*scale),(.047*scale,.025*scale,.048*scale),c,b,12)
    ball('Flower center',(p[0],p[1]-.025,p[2]),(.032*scale,.022*scale,.032*scale),'f6d779',b,12)

def braid(x,y,z,length,color,b='Hair'):
    for i in range(8):
        ball('Braided hair',(x+(.025 if i%2 else -.025),y,z-i*length/8),(.09-i*.005,.075,.09),color,b,12)
    bow((x,y-.03,z-length),'e7b56c',b,.5)

def aya_twintails(color):
    """Two low butterfly-tied hair masses reaching the ankle (z=.12-.18)."""
    ball('Aya back hair cap',(0,.12,1.51),(.40,.26,.31),color,'Head')
    # Each tail is one closed smooth volume, separated from the central kimono.
    # Horizontal rings keep a readable broad hair silhouette rather than a braid.
    profile=[(.36,.13,1.29,.07,.065),(.41,.17,1.17,.10,.08),
             (.46,.21,1.02,.13,.10),(.49,.25,.84,.145,.11),
             (.50,.28,.65,.15,.115),(.55,.29,.47,.17,.12),
             (.59,.28,.31,.16,.105),(.56,.26,.19,.12,.075),
             (.62,.24,.13,.07,.045),(.68,.23,.18,.012,.012)]
    for sign in [-1,1]:
        petal('Hair swept to low tie',(sign*.12,.20,1.70),(sign*.36,.16,1.27),.19,color,'Head')
        vertices=[];faces=[];count=16
        for x,y,z,rx,ry in profile:
            for j in range(count):
                t=j*math.tau/count
                vertices.append((sign*(x+rx*math.cos(t)),y+ry*math.sin(t),z))
        for row in range(len(profile)-1):
            for j in range(count):
                a=row*count+j;b=row*count+(j+1)%count
                face=(a,b,b+count,a+count)
                faces.append(tuple(reversed(face)) if sign==1 else face)
        caps=[tuple(range(count)),tuple(reversed(range((len(profile)-1)*count,len(profile)*count)))]
        faces += caps if sign==1 else [tuple(reversed(face)) for face in caps]
        data=bpy.data.meshes.new('Ankle-length twin tail');data.from_pydata(vertices,[],faces);data.update()
        obj=bpy.data.objects.new('Aya ankle twin tail '+str(sign),data);bpy.context.collection.objects.link(obj)
        finish(obj,obj.name,color,'Hair')
        smooth=obj.modifiers.new('Flowing hair surface','SUBSURF');smooth.levels=2
        bpy.context.view_layer.objects.active=obj;bpy.ops.object.modifier_apply(modifier=smooth.name)
        # Cool strand ridges distinguish the white hair from the white fabric.
        for offset in [-.045,.045]:
            points=[(sign*(x+offset),y-ry-.004,z) for x,y,z,rx,ry in profile[1:-1]]
            tube('Twin tail strand',points,.009,'d8dfed','Hair')


def aya_parasol():
    """A rigid hand-held flower canopy, separate from Aya's ankle-length twin tails."""
    grip=Vector((-.52,-.11,.77)); hub=Vector((-.20,.25,2.00))
    axis=(hub-grip).normalized()
    u=Vector((1,0,0)); u=(u-axis*u.dot(axis)).normalized(); v=axis.cross(u)
    tube('Aya parasol shaft',[grip-axis*.12,hub+axis*.16],.019,'f2eee6','Hand.L')
    ball('Aya parasol hub',hub,(.065,.065,.045),'c7cfdd','Hand.L')
    for petal_index in range(8):
        center=petal_index*math.tau/8
        vertices=[]; faces=[]
        for ring_index in range(7):
            r=ring_index/6
            for column in range(7):
                angle=center+(column/6-.5)*math.tau/8*1.10
                edge=.76*(.87+.13*math.cos((column/6-.5)*math.pi))
                lift=.18*(1-r*r)-.12*r*r+ .07*r**6
                p=hub+(u*math.cos(angle)+v*math.sin(angle))*r*edge+axis*lift
                vertices.append(tuple(p))
        for row in range(6):
            for column in range(6):
                a=row*7+column; faces.append((a,a+1,a+8,a+7))
        data=bpy.data.meshes.new('Flower parasol panel');data.from_pydata(vertices,[],faces);data.update()
        obj=bpy.data.objects.new('Aya parasol petal',data);bpy.context.collection.objects.link(obj)
        finish(obj,'Aya parasol petal','f5f6fc','Hand.L')
        obj.data.materials.append(mat('aab8d1'))
        solid=obj.modifiers.new('Canopy fabric thickness','SOLIDIFY');solid.thickness=.012;solid.material_offset=1
        bpy.context.view_layer.objects.active=obj;bpy.ops.object.modifier_apply(modifier=solid.name)
        rib=[]
        for i in range(7):
            r=i/6
            rib.append(hub+(u*math.cos(center)+v*math.sin(center))*r*.75+axis*(.18*(1-r*r)-.12*r*r+.07*r**6-.015))
        tube('Aya parasol rib',rib,.008,'dddde6','Hand.L')
    ball('Aya parasol finial',hub+axis*.22,(.035,.035,.06),'e4decd','Hand.L')

def make_rig():
    arm=bpy.data.armatures.new('TSB_Weighted_Rig')
    rig=bpy.data.objects.new('TSB_Rig',arm); bpy.context.collection.objects.link(rig)
    bpy.context.view_layer.objects.active=rig; rig.select_set(True); bpy.ops.object.mode_set(mode='EDIT')
    bones=[('Root',(0,0,0),None),('Hips',(0,0,.72),'Root'),('Torso',(0,0,.82),'Hips'),('Head',(0,0,1.22),'Torso'),
           ('Hair',(0,.14,1.62),'Head'),('Cape',(0,.13,1.06),'Torso'),('Tail',(0,.15,.65),'Hips'),('Mount',(0,0,.3),'Root')]
    for side,sign in [('L',-1),('R',1)]:
        bones += [('UpperArm.'+side,(sign*.29,0,1.08),'Torso'),('Forearm.'+side,(sign*.43,-.01,.91),'UpperArm.'+side),
                  ('Hand.'+side,(sign*.51,-.04,.77),'Forearm.'+side),('Thigh.'+side,(sign*.145,0,.68),'Hips'),
                  ('Shin.'+side,(sign*.145,0,.38),'Thigh.'+side),('Foot.'+side,(sign*.145,-.04,.12),'Shin.'+side)]
    for name,p,parent in bones:
        bone=arm.edit_bones.new(name); bone.head=p; bone.tail=Vector(p)+Vector((0,0,.12))
        if parent: bone.parent=arm.edit_bones[parent]
    bpy.ops.object.mode_set(mode='OBJECT'); rig.select_set(False)
    rig.show_in_front=True
    return rig

def character_model(index):
    global RIG,PARTS,MATS
    bpy.ops.object.select_all(action='SELECT'); bpy.ops.object.delete(use_global=False)
    PARTS=[];MATS={}; RIG=make_rig()
    hair,cloth,trim,eye,style=SPECS[index]
    skin='bc8274' if index==4 else 'ffe2d1'
    dark='3c303d'; white='fffaf4'
    ball('Round cheek head',(0,0,1.49),(.405,.307,.335),skin)
    # Face lies on the forward (-Y) surface. Layered irises and lashes read from FPS distances.
    sleepy=index in [3,8,12,13]
    for side in [-1,1]:
        x=side*.165
        h=.065 if sleepy else .1
        ball('Eye outline',(x,-.286,1.48),(.098,.012,h+.022),dark)
        ball('Eye white',(x,-.295,1.475),(.083,.008,h),white)
        iris= 'b48ce5' if index==14 and side==1 else eye
        ball('Colored iris',(x,-.303,1.47),(.055,.006,h*.88),iris)
        ball('Pupil',(x,-.309,1.48),(.021,.004,h*.6),dark)
        ball('Eye glint',(x-.018,-.314,1.50),(.018,.004,.022),white,seg=12)
        tube('Upper eyelash',[(x-.083,-.302,1.49+h*.8),(x,-.310,1.50+h*.8),(x+.085,-.300,1.49+h*.8)],.009,dark)
        ball('Blush',(side*.29,-.237,1.365),(.073,.018,.043),'f19aa5')
        if index in [0,2,3,6,7,8,13]:
            petal('Pointed ear',(side*.34,.015,1.47),(side*.56,.025,1.57),.085,skin)
        else: ball('Ear',(side*.38,0,1.46),(.075,.065,.1),skin)
    tube('Small smile',[(-.048,-.299,1.325),(0,-.321,1.31),(.048,-.299,1.325)],.009,dark)
    # Sculpted cap, bangs and distinct strand volumes.
    ball('Hair crown',(0,.038,1.67),(.415,.30,.21),hair)
    for j in range(5):
        x=(j-2)*.14
        petal('Front bang',(x,-.18,1.76),(x+(.04 if j%2 else -.035),-.275,1.55+(.03 if j%2 else 0)),.105,hair)
    for sign in [-1,1]:
        petal('Face lock',(sign*.32,-.10,1.68),(sign*.35,-.11,1.28),.10,hair)
    if style in ['long','twintails','flame']:
        for j in range(7):
            x=(j-3)*.115
            end=.37 if index in [0,2,3,9] else .59
            petal('Long back hair',(x,.18,1.66),(x*1.18,.25,end),.11,hair,'Hair')
            if index in [9,12]: ball('Hair color tip',(x*1.18,.25,end+.05),(.11,.065,.19),'b6b4df' if index==9 else 'eab7cb','Hair')
    if style in ['pony','sidepony']:
        x=.35 if style=='sidepony' else 0
        petal('High ponytail',(x,.23,1.83),(x+.09,.34,.6),.18,hair,'Hair')
        bow((x,.23,1.82),trim,'Head',.8)
    if style in ['braid','braids']:
        for sign in ([-1,1] if style=='braids' else [1]): braid(sign*.34,.07,1.30,.72,hair)
    if style=='buns':
        for sign in [-1,1]:
            ball('Candy bun',(sign*.39,.05,1.67),(.15,.11,.17),hair)
            ring('Candy swirl',(sign*.39,-.06,1.67),.085,.1,'d9a5d9',r=.025)
            tube('Curly pigtail',[(sign*.4,.08,1.5),(sign*.45,.12,1.28),(sign*.37,.1,1.17)],.035,hair,'Hair')
            flower((sign*.31,-.13,1.82),'f49eb7',scale=.55)
    # Biped torso and articulated limbs.
    ball('Torso',(0,0,.97),(.245,.16,.245),skin if index==3 else white if index==7 else cloth,'Torso')
    ball('Neck',(0,0,1.2),(.09,.09,.1),skin,'Torso')
    long_dress=index in [0,4,9]
    if index not in [3,7,12,13]:
        taper('Flared skirt',(0,0,.67 if not long_dress else .52),.32 if not long_dress else .41,.20,.28 if not long_dress else .8,hair if index==5 else cloth,'Hips')
        taper('Skirt hem',(0,0,.535 if not long_dress else .135),.331 if not long_dress else .421,.33 if not long_dress else .42,.045,trim,'Hips')
    elif index==3:
        ball('Bodysuit fitted hips',(0,0,.72),(.22,.155,.13),trim,'Hips')
        ball('Bodysuit white lower center',(0,-.145,.73),(.10,.022,.12),cloth,'Hips')
    else: box('Shorts or trousers',(0,0,.69),(.41,.28,.22),cloth if index==7 else '78605b' if index==13 else trim,'Hips')
    for side,sign in [('L',-1),('R',1)]:
        x=sign*.145
        legcolor=trim if index in [2,3,7,11,13,14] else skin
        if index==2:legcolor=white
        if index==3:legcolor=skin
        if index==7:legcolor=cloth
        if index==13:legcolor='78605b'
        ball('Thigh '+side,(x,0,.63) if index==3 else (x,0,.52),(.10,.10,.10) if index==3 else (.105,.105,.21),legcolor,'Thigh.'+side)
        ball('Shin '+side,(x,0,.26),(.09,.09,.18),trim if index==3 else legcolor,'Shin.'+side)
        ball('Shoe '+side,(x,-.055,.085),(.112,.165,.09),white if index in [2,3] else trim if index not in [0,7,8,13] else dark,'Foot.'+side)
        petal(('Bare upper arm ' if index==3 else 'Sleeve ')+side,(sign*.25,0,1.09),(sign*.39,0,.94),.08 if index==3 else .115,skin if index==3 else white if index==4 else trim if index in [5,13] else cloth,'UpperArm.'+side)
        petal('Forearm '+side,(sign*.40,-.01,.94),(sign*.50,-.04,.79),.073,white if index==4 else skin if index not in [5,7,9,14] else trim,'Forearm.'+side)
        ball('Mitten hand '+side,(sign*.52,-.045,.765),(.082,.063,.083),skin,'Hand.'+side)
        ball('Thumb '+side,(sign*.48,-.09,.78),(.035,.035,.048),skin,'Hand.'+side,12)
    # Individual costumes / accessories from the fifteen guides.
    if index==0:
        bow((0,-.17,1.10),dark,'Torso',.7)
        for sign in [-1,1]:
            tube('Swept dragon horn',[(sign*.27,.06,1.8),(sign*.39,.06,1.98),(sign*.33,.18,2.04)],.055,'3e3d4e')
            for j in range(3): petal('Metal dragon wing',(sign*.24,.23,1.08),(sign*(.57+j*.10),.22,1.45-j*.19),.085,'44445c','Cape')
            petal('White gown petal',(sign*.15,-.08,.98),(sign*.35,-.02,.30),.18,white,'Hips')
            ball('Red earring',(sign*.43,-.02,1.32),(.035,.035,.07),'c34654')
    if index in [1,10]:
        ball('Straw hat brim',(0,.015,1.82),(.60,.43,.052),'d6ad7d' if index==1 else 'e8bb65')
        ball('Straw hat crown',(0,.03,1.87),(.36,.29,.15),'d6ad7d' if index==1 else 'e8bb65')
        flower((.35,-.29,1.86),'e8ce66' if index==1 else 'e9a4b9',scale=1.0)
        if index==1:
            box('White waistband',(0,-.14,.81),(.41,.045,.07),white,'Hips')
            flower((0,-.18,1.1),'ead06c','Torso',.6)
        else:
            ball('Hat frog',(0,.02,2.01),(.105,.09,.065),'9bcf76')
            for sign in [-1,1]: ball('Hat frog eye',(sign*.046,-.046,2.06),(.02,.02,.026),dark)
            ball('Frog mount',(0,0,.38),(.65,.42,.34),'98cd6b','Mount')
            ball('Frog muzzle',(0,-.29,.51),(.55,.20,.17),'addc7d','Mount')
            for sign in [-1,1]:
                ball('Frog eye dome',(sign*.28,-.19,.70),(.11,.10,.12),'98cd6b','Mount')
                ball('Frog eye',(sign*.28,-.286,.72),(.028,.014,.04),dark,'Mount')
                ball('Frog cheek',(sign*.47,-.28,.47),(.14,.08,.10),'eea79f','Mount')
                ball('Frog haunch',(sign*.5,.14,.23),(.24,.25,.19),'98cd6b','Mount')
                for j in range(3): ball('Frog toe',(sign*(.35+j*.07),-.31,.10),(.055,.13,.045),'98cd6b','Mount',12)
            tube('Frog smile',[(-.3,-.468,.53),(0,-.49,.49),(.3,-.468,.53)],.012,'55814a','Mount')
            box('Frog yellow sash',(0,-.36,.25),(.9,.09,.09),'efc260','Mount')
    if index in [2,8]:
        for sign in [-1,1]:
            for j in range(3): petal('Fairy wing',(sign*.20,.16,1.0),(sign*(.48+j*.07),.21,1.2-j*.08),.08,white,'Cape')
        if index==2:
            taper('Gold crown',(0,0,1.86),.18,.21,.08,'e9bf72','Head',8)
            for sign in [-1,0,1]: taper('Crown point',(sign*.13,0,1.94),.065,0,.13,'e9bf72','Head',4)
            ball('Crown jewel',(0,-.18,1.90),(.04,.025,.06),'7cb8dd')
            box('Royal cape',(0,.19,.68),(.6,.09,.83),'b64a5b','Cape')
            bow((0,-.17,1.11),'b64a5b','Torso',.9)
            braid(-.28,-.14,1.29,.7,hair)
        else:
            ball('Collector cap',(0,.04,1.88),(.29,.24,.09),cloth)
            box('Cap band',(0,-.19,1.88),(.21,.05,.12),white,'Head')
            box('Leather backpack',(0,.28,.86),(.43,.19,.55),'987358','Cape')
            for sign in [-1,1]: box('Backpack pocket',(sign*.12,.39,.73),(.17,.06,.17),'b08a67','Cape')
            ball('Blue apron',(0,-.16,.79),(.23,.03,.26),cloth,'Torso')
    if index==3:
        # User costume references: fitted sleeveless bodysuit, crossed neck straps,
        # bare arms, wrist bands and thigh-high stockings. Preserve chibi anatomy.
        # Follow the body surface, with one material panel rather than a raised oval.
        garment_vertices=[]; garment_faces=[]
        for z in [.76,.81,.87,.94,1.00,1.06,1.10]:
            radius=math.sqrt(1-((z-.97)/.245)**2)
            for j in range(32):
                angle=j*math.tau/32
                garment_vertices.append((.251*radius*math.sin(angle),-.166*radius*math.cos(angle),z))
        for row in range(6):
            for j in range(32):
                k=(j+1)%32
                garment_faces.append((row*32+j,row*32+k,(row+1)*32+k,(row+1)*32+j))
        garment_mesh=bpy.data.meshes.new('Fitted bodysuit surface')
        garment_mesh.from_pydata(garment_vertices,[],garment_faces)
        garment=bpy.data.objects.new('Fitted bodysuit surface',garment_mesh)
        bpy.context.collection.objects.link(garment)
        finish(garment,'Fitted bodysuit surface',trim,'Torso')
        garment.data.materials.append(mat(cloth))
        for polygon in garment.data.polygons:
            if math.cos((polygon.index%32+.5)*math.tau/32)>.90:
                polygon.material_index=1
        taper('Bodysuit white neck band',(0,0,1.20),.096,.096,.038,cloth,'Torso')
        for sign in [-1,1]:
            tube('Bodysuit crossed halter strap',[(sign*.12,-.107,1.105),(-sign*.052,-.101,1.18)],.012,trim,'Torso')
        for side,sign in [('L',-1),('R',1)]:
            ball('White wrist band '+side,(sign*.492,-.038,.802),(.076,.067,.022),cloth,'Forearm.'+side)
            # The upper stocking ends below the uncovered upper thigh.
            taper('Fitted upper stocking '+side,(sign*.145,0,.47),.10,.113,.24,trim,'Thigh.'+side)
            tube('Upper stocking white side piping '+side,[(sign*.238,-.04,.36),(sign*.251,-.04,.58)],.006,cloth,'Thigh.'+side)
            tube('Lower stocking white side piping '+side,[(sign*.217,-.04,.13),(sign*.232,-.04,.38)],.006,cloth,'Shin.'+side)
        # Thick crescent in the X/Z plane, with an open front and padded seat.
        points=[];lining=[]
        for i in range(25):
            t=math.radians(-115+i*230/24)
            points.append((.66*math.cos(t),.28,1.04+.86*math.sin(t)))
            lining.append((.57*math.cos(t),.12,1.04+.76*math.sin(t)))
        tube('Continuous crescent shell',points,.15,'54576b','Mount')
        tube('Padded inner crescent',lining,.07,'edeaf0','Mount')
        ball('Moon cushion',(0,.20,.67),(.5,.27,.17),'edebee','Mount')
        for sign in [-1,1]: ball('Moon lamp',(sign*.54,.01,.65),(.08,.03,.09),white,'Mount')
        ball('Companion robot',(-.59,.10,1.88),(.14,.10,.14),'e9e8e9','Mount')
        ball('Robot face',(-.59,-.002,1.88),(.10,.025,.10),'373c50','Mount')
        for x in [-.62,-.56]: ball('Robot eye',(x,-.03,1.90),(.012,.008,.025),white,'Mount',12)
    if index==4:
        # The high petal silhouette in the source is a parasol, not piled-up hair.
        aya_twintails(hair)
        box('Aya black forehead patch',(-.12,-.24,1.69),(.10,.025,.16),'343f43','Head',.015)
        aya_parasol()
        box('Black obi',(0,-.14,.89),(.44,.08,.16),'343f43','Torso')
        for z in [.83,.95]: box('Obi gold edge',(0,-.19,z),(.45,.018,.022),'dab873','Torso')
        for sign in [-1,1]:
            bow((sign*.37,.015,1.27),'b9deea','Head',1.1)
            petal('Kimono panel',(sign*.12,-.12,.86),(sign*.22,-.17,.19),.16,white,'Hips')
    if index==5:
        bow((0,-.18,1.09),'df769a','Torso',.75)
        for j,c in enumerate(['f6d57b','bca1e4','8de0e1','ef9cba']):
            ball('Candy button',((j-1.5)*.12,-.18,.76),(.06,.035,.055),c,'Hips')
            box('Candy hairpin',((j-1.5)*.13,-.30,1.64),(.075,.028,.035),c,'Head',.01)
    if index==6:
        ball('Sebastian plush hood',(0,.04,1.91),(.47,.32,.28),white)
        for sign,c in [(-1,'99ced0'),(1,'e8adc7')]:
            ball('Plush ear',(sign*.34,.03,2.11),(.15,.1,.15),c)
            ball('Plush eye',(sign*.15,-.257,1.97),(.04,.025,.048),dark)
            ball('Plush cheek',(sign*.33,-.195,1.88),(.11,.04,.09),'efa0af')
        ball('Plush nose',(0,-.29,1.92),(.043,.02,.025),dark)
        tube('Heart tuft',[(0,.02,2.16),(-.10,.02,2.30),(-.04,.02,2.36),(0,.02,2.30),(.07,.02,2.35),(.11,.02,2.28),(0,.02,2.16)],.025,'ce9bbd')
        petal('Quilted stuffed tail',(0,.23,.75),(.22,.53,.26),.20,'e3a4c3','Tail')
        box('Purple cape',(0,.17,.85),(.5,.06,.42),'8c5b88','Cape')
        bow((0,-.19,1.10),'853c51','Torso',.9)
    if index==7:
        ring('Geometric halo',(0,.17,1.84),.47,.35,'d6b769',r=.012)
        for sign in [-1,1]:
            taper('Temple fin',(sign*.33,.02,1.85),.055,0,.33,'3c4057','Head',4)
            ball('Gold ear disc',(sign*.42,.01,1.51),(.045,.1,.13),'d6b769')
            box('Oversized coat sleeve',(sign*.43,.01,.91),(.22,.25,.34),cloth,'Forearm.'+('L' if sign<0 else 'R'))
        box('Long split coat',(0,.18,.62),(.55,.08,.78),cloth,'Cape')
        box('Gold coat trim',(0,.229,.62),(.035,.014,.76),trim,'Cape')
    if index==9:
        for sign in [-1,1]:
            tube('Spectral ribbon',[(sign*.28,.12,.9),(sign*.56,.2,.6),(sign*.43,.2,.23)],.065,'6a6a9e','Cape')
            ball('Ghost flame',(sign*.65,.03,1.35),(.07,.065,.13),'8ba7ed','Cape')
            ball('Ghost flame core',(sign*.65,-.03,1.33),(.035,.025,.07),'c8f4ff','Cape')
        for j in [-1,0,1]: petal('Gown front petals',(j*.15,-.11,.8),(j*.23,-.20,.23),.14,'b2b7eb','Hips')
    if index==11:
        for sign in [-1,1]:
            petal('Floppy dog ear',(sign*.31,.03,1.79),(sign*.52,.03,1.62),.13,hair)
            bow((sign*.39,-.03,1.79),'edc15d','Head',.55)
            for z in [.22,.33,.44]: box('Striped sock',(sign*.145,-.083,z),(.17,.025,.055),'bd956b','Shin.'+('L' if sign<0 else 'R'),.01)
        bow((0,-.18,1.09),'edbd55','Torso',.85)
        ball('Collar bell',(0,-.13,1.19),(.045,.04,.045),'e5b64c','Torso')
        ball('Puppy bag',(.22,-.19,.75),(.14,.07,.13),'bb956d','Hips')
        petal('Dog tail',(0,.18,.68),(.2,.43,.78),.13,hair,'Tail')
    if index==12:
        for sign in [-1,1]:
            cat_ear(sign,hair,'e9afbd')
        tube('Cat tail',[(0,.17,.67),(.27,.38,.69),(.37,.35,.96)],.035,'e9b6c9','Tail')
        box('Pajama collar',(0,-.13,1.14),(.25,.04,.035),'ecb4c3','Torso',.01)
    if index==13:
        for sign in [-1,1]: ring('Round glasses',(sign*.165,-.373,1.46),.11,.093,'695c5c',r=.009)
        tube('Glasses bridge',[(-.05,-.371,1.48),(0,-.38,1.49),(.05,-.371,1.48)],.008,'695c5c')
        box('Cross pin vertical',(.24,-.23,1.72),(.03,.03,.10),'cadc77','Head',.005)
        box('Cross pin horizontal',(.24,-.25,1.72),(.09,.03,.03),'cadc77','Head',.005)
        box('Cream knit bib',(0,-.15,1.07),(.3,.06,.18),'e9d8bd','Torso')
        for x in [-.09,-.045,0,.045,.09]: tube('Knit cable',[(x,-.19,1.0),(x+.014,-.19,1.06),(x,-.19,1.15)],.008,'c7b49b','Torso')
        box('ID badge',(.15,-.18,.94),(.06,.025,.08),white,'Torso',.005)
        box('White lab coat',(0,.15,.64),(.54,.09,.65),white,'Cape')
    if index==14:
        ball('Side dragon mask',(.30,-.05,1.78),(.16,.10,.23),'3b424c')
        for x in [.22,.36]: taper('Mask horn',(x,-.02,1.98),.045,0,.15,'3b424c','Head',4)
    # Shared torso ornaments that preserve character palette.
    if index in [2,7,14]: ball('Chest jewel',(0,-.19,1.04),(.065,.033,.075),'83b4df','Torso')
    if index==14: taper('Bell shoulder mantle',(0,0,1.01),.34,.16,.29,cloth,'Torso')
    if index in [0,2,6,9]:
        for j in range(12):
            t=j*math.tau/12
            ball('Scalloped hem',(.32*math.sin(t),.22*math.cos(t),.53 if index in [2,6] else .15),(.055,.045,.04),trim,'Hips',12)
    # Combine all parts into one skinned mesh. Every vertex retains a named bone weight.
    bpy.ops.object.select_all(action='DESELECT')
    for obj in PARTS: obj.select_set(True)
    bpy.context.view_layer.objects.active=PARTS[0]
    bpy.ops.object.join(); mesh=bpy.context.object; mesh.name=SLUGS[index]+'_SkinnedMesh'
    modifier=mesh.modifiers.new('TSB skin','ARMATURE'); modifier.object=RIG
    mesh.parent=RIG
    return mesh

def combat_rotations(index, name, strike):
    """Socket .L is anatomical right; Aya already holds her parasol in that hand."""
    melee=index in [0,3,6,9,12]
    primary,secondary,sign=('R','L',1) if index==4 or name=='Ultimate' else ('L','R',-1)
    rotations={
        'Torso':(.12*strike,sign*(.24 if melee else -.12)*strike,0),
        'UpperArm.'+primary:((-1.3 if melee else -.9)*strike,0,sign*.18*strike),
        'Forearm.'+primary:(-.55*strike,0,0),
        'UpperArm.'+secondary:(0,0,0),
        'Forearm.'+secondary:(0,0,0),
    }
    if name!='Attack': rotations['UpperArm.'+secondary]=(-.8*strike,0,-sign*.35*strike)
    if name=='Ultimate':
        rotations['UpperArm.L']=(-1.7*strike,0,-.25*strike)
        rotations['UpperArm.R']=(-1.7*strike,0,.25*strike)
        # This two-arm gesture has no dominant hand; retain its authored torso turn.
        rotations['Torso']=(.12*strike,(.24 if melee else -.12)*strike,0)
    return rotations


def animate(index):
    rig=RIG; rig.animation_data_create()
    action=bpy.data.actions.new('TSB_Motions'); rig.animation_data.action=action
    for pb in rig.pose.bones: pb.rotation_mode='XYZ'
    for name,(start,end) in CLIPS.items():
        for f in range(start,end+1):
            t=(f-start)/(end-start); cycle=t*math.tau
            for p in rig.pose.bones: p.rotation_euler=(0,0,0); p.location=(0,0,0); p.scale=(1,1,1)
            def rot(b,x=0,y=0,z=0): rig.pose.bones[b].rotation_euler=(x,y,z)
            hover=index in [0,3,7,9]
            bob=.015*math.sin(cycle)
            if name in ['Walk','Run']:
                pace=.68 if name=='Run' else .42
                bob=.045*abs(math.sin(cycle))
                for side,sign in [('L',1),('R',-1)]:
                    rot('Thigh.'+side,pace*math.sin(cycle)*sign if not hover else .08)
                    rot('Shin.'+side,max(0,-math.sin(cycle)*sign)*.55 if not hover else .08)
                    rot('UpperArm.'+side,-pace*.65*math.sin(cycle)*sign)
                    rot('Forearm.'+side,-.2)
                rot('Torso',.10 if name=='Run' else .04)
            elif name=='Jump':
                bob=.08*math.sin(math.pi*t)
                rot('Thigh.L',-.35*math.sin(math.pi*t)); rot('Thigh.R',.3*math.sin(math.pi*t))
                rot('UpperArm.L',0,0,-.5*math.sin(math.pi*t)); rot('UpperArm.R',0,0,.5*math.sin(math.pi*t))
            elif name in ['Attack','Skill','Ultimate']:
                strike=math.sin(math.pi*t)**2
                for bone,angles in combat_rotations(index,name,strike).items(): rot(bone,*angles)
                if name=='Ultimate':
                    bob=.07*strike
            elif name=='Dance':
                # Four second seamless original dance, with per-character timing/gesture variants.
                beat=cycle*2; swing=math.sin(beat)
                bob=.055*(1-math.cos(beat*2))
                rig.pose.bones['Hips'].location.x=.085*swing
                rot('Hips',0,.12*swing,.10*swing)
                rot('Torso',.06*math.sin(beat*2),-.2*swing,-.12*swing)
                rot('Head',.06*math.cos(beat),.13*swing,.09*swing)
                for side,sign in [('L',-1),('R',1)]:
                    rot('UpperArm.'+side,-.2+.35*math.sin(beat+sign*.7),0,sign*(.5+.3*math.cos(beat)))
                    rot('Forearm.'+side,-.65-.35*math.cos(beat+sign))
                    rot('Hand.'+side,0,.25*swing,sign*.15)
                    if not hover:
                        rot('Thigh.'+side,.16*math.sin(beat)*sign,0,sign*.08)
                        rot('Shin.'+side,.14*(1+math.cos(beat+sign*math.pi/2)))
                if index%3==1: rot('UpperArm.R',-.7+.3*swing,0,.85+.2*swing)
                if index%3==2: rot('Head',.1*math.sin(beat*2),.25*swing,.12*swing)
            elif name=='Hurt': rot('Torso',-.22*math.sin(math.pi*t))
            elif name=='Select': bob=selection_pose(index,t,rot,rig)
            elif name=='Death':
                rot('Hips',min(t*2,1)*1.25)
                rig.pose.bones['Hips'].location.z=-.3*min(t*2,1)
            if name!='Death': rig.pose.bones['Hips'].location.z=bob+(.08 if hover else 0)
            rot('Hair',.04*math.sin(cycle+.4),0,.035*math.sin(cycle))
            rot('Cape',.04*math.sin(cycle+.8))
            rot('Tail',.06,0,.30*math.sin(cycle*2))
            if index==3:
                # Pilot stays seated in her moving crescent chair.
                rot('Thigh.L',-1.15);rot('Thigh.R',-1.15);rot('Shin.L',.8);rot('Shin.R',.8)
                rig.pose.bones['Mount'].location.z=.04*math.sin(cycle)
                if name=='Dance': rot('Mount',0,.08*math.sin(cycle*2),.08*math.sin(cycle*2))
            if index==10:
                rig.pose.bones['Mount'].location.z=bob*.8
                rot('Mount',0,0,.05*math.sin(cycle*2) if name=='Dance' else 0)
            if index==4:
                # One hand keeps its grip while the torso and free arm carry the gesture.
                rot('UpperArm.L');rot('Forearm.L');rot('Hand.L')
            for pb in rig.pose.bones:
                pb.keyframe_insert(data_path='rotation_euler',frame=f,group=pb.name)
                pb.keyframe_insert(data_path='location',frame=f,group=pb.name)
    return action

def export(index,mesh):
    slug=SLUGS[index];scene=bpy.context.scene
    scene.render.fps=30; scene.frame_start=0;scene.frame_end=528
    scene.frame_set(0)
    # The editable .blend embeds the generated sheet as a hidden image reference.
    image=bpy.data.images.load(str(ROOT/'assets/turnarounds'/f'{slug}.png'))
    image.pack()
    reference=bpy.data.objects.new('Turnaround_REFERENCE_only',None)
    bpy.context.collection.objects.link(reference);reference.empty_display_type='IMAGE';reference.data=image
    reference.empty_display_size=3;reference.location=(3,1,1);reference.hide_render=True;reference.hide_viewport=True
    scene['source_guide']='assets/turnarounds/'+slug+'.png'
    scene['interpretation']='Fan-made MVP 3D adaptation; side/back are inferred.'
    scene['clips']=json.dumps(CLIPS)
    bpy.ops.object.camera_add(location=(3,-5,2.5))
    camera=bpy.context.object;camera.name='Preview camera'
    camera.rotation_euler=(Vector((0,0,1.08))-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.type='ORTHO';camera.data.ortho_scale=2.95 if index==4 else 2.65;scene.camera=camera
    scene.render.engine='CYCLES';scene.cycles.samples=12;scene.cycles.device='CPU'
    scene.world.color=(.65,.65,.65)
    for p,power,size in [((2,-4,5),450,4),((-3,-1,3),280,3),((0,3,4),450,3)]:
        bpy.ops.object.light_add(type='AREA',location=p)
        lamp=bpy.context.object;lamp.data.energy=power;lamp.data.shape='DISK';lamp.data.size=size
        lamp.rotation_euler=(Vector((0,0,1))-lamp.location).to_track_quat('-Z','Y').to_euler()
    scene.render.resolution_x=512;scene.render.resolution_y=512;scene.render.resolution_percentage=100
    scene.render.image_settings.file_format='PNG';scene.render.film_transparent=True
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/f'{slug}.blend'))
    sys.path.insert(0,str(ROOT/'scripts'))
    from export_fyrox_fbx import export_fbx
    export_fbx(mesh,RIG,OUT/f'{slug}.fbx')
    scene.render.filepath=str(OUT/f'{slug}-preview.png')
    bpy.ops.render.render(write_still=True)
    return {'id':index,'slug':slug,'blend':f'assets/characters/{slug}.blend','model':f'assets/characters/{slug}.fbx',
            'vertices':len(mesh.data.vertices),'triangles':sum(len(p.vertices)-2 for p in mesh.data.polygons),
            'bones':len(RIG.data.bones),'clips':CLIPS,'fps':30,'guide':f'assets/turnarounds/{slug}.png'}

def main():
    args=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else []
    ids=[SLUGS.index(s) for s in args] if args else list(range(15))
    records=[]
    for i in ids:
        mesh=character_model(i);animate(i);records.append(export(i,mesh))
        (OUT/'build-progress.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
        print('TSB_MODEL_OK',SLUGS[i],records[-1]['vertices'],flush=True)
    if len(records)==15: (OUT/'manifest.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
    print('TSB_ART_BUILD_COMPLETE',len(records),flush=True)

if __name__=='__main__':
    try: main()
    except Exception:
        import traceback
        (OUT/'build-error.txt').write_text(traceback.format_exc(),encoding='utf-8')
        raise
