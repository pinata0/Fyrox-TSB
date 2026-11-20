"""Fixed segment IK and rigid sleeve sections, with a small rounded elbow seam."""
import bpy, math
from mathutils import Vector
from pose_id_rider import world_rotation

def solve(rig, side, target, lengths, base):
    names=[f'{part}.{side}' for part in ['UpperArm','Forearm','Hand']]
    # Animated arm length/scale is forbidden. Only rotations solve the reach.
    for name in names:
        p=rig.pose.bones[name];location,_,scale=base[name].decompose()
        p.location=location;p.scale=scale
    bpy.context.view_layer.update()
    shoulder=rig.pose.bones[names[0]].head.copy()
    a,b=lengths[side];direction=target-shoulder
    distance=max(abs(a-b)+.005,min(direction.length,(a+b)*.985))
    direction.normalize();target=shoulder+direction*distance
    # Stable anatomical elbow pole; avoid flipping on a fully straight arm.
    pole=Vector((-1 if side=='L' else 1,.20,-.15))
    bend=(pole-direction*pole.dot(direction)).normalized()
    along=(a*a-b*b+distance*distance)/(2*distance)
    goal=shoulder+direction*along+bend*math.sqrt(max(0,a*a-along*along))
    elbow=rig.pose.bones[names[1]].head.copy()
    delta=(elbow-shoulder).rotation_difference(goal-shoulder)
    world_rotation(rig,names[0],delta@rig.pose.bones[names[0]].matrix.to_quaternion())
    elbow=rig.pose.bones[names[1]].head.copy();wrist=rig.pose.bones[names[2]].head.copy()
    world_rotation(rig,names[1],(wrist-elbow).rotation_difference(target-elbow)@rig.pose.bones[names[1]].matrix.to_quaternion())
    for i in range(2):
        actual=(rig.pose.bones[names[i+1]].head-rig.pose.bones[names[i]].head).length
        assert abs(actual-lengths[side][i])<.00002,(names[i],actual,lengths[side][i])

def rebuild_sleeves(rig):
    """No long blended strip spans the elbow: two sleeves meet at a rigid joint."""
    for side in ['L','R']:
        s,e,w=[rig.data.bones[f'{p}.{side}'].head_local.copy() for p in ['UpperArm','Forearm','Hand']]
        for suffix in ['RefinedSleeve','RefinedViewSleeve']:
            obj=bpy.data.objects[f'chloe_{suffix}.{side}'];materials=list(obj.data.materials)
            vertices=[];faces=[];groups=[];material_ids=[]
            for start,end,bone,ra,rb in [(s,e,f'UpperArm.{side}',.055,.052),(e,w,f'Forearm.{side}',.052,.037)]:
                axis=(end-start).normalized();cross=axis.cross(Vector((0,1,0))).normalized();up=axis.cross(cross).normalized()
                rings=[]
                for j,t in enumerate([0,.08,.25,.5,.75,.90,.97,1]):
                    # White cuff is a short constant-length part of the forearm.
                    center=start+(end-start)*t;radius=ra+(rb-ra)*t
                    ring=[]
                    for k in range(24):
                        angle=k*math.tau/24;ring.append(len(vertices));vertices.append(center+(cross*math.cos(angle)+up*math.sin(angle))*radius);groups.append(bone)
                    if rings:
                        for k in range(24):faces.append((rings[-1][k],rings[-1][(k+1)%24],ring[(k+1)%24],ring[k]));material_ids.append(int(bone.startswith('Forearm') and t>.90))
                    rings.append(ring)
                faces.extend([tuple(reversed(rings[0])),tuple(rings[-1])]);material_ids.extend([0,int(bone.startswith('Forearm'))])
            # Rounded elbow hides the overlap without a stretchy skin bridge.
            base=len(vertices);radius=.054
            for j in range(9):
                polar=j*math.pi/8
                for k in range(16):
                    a=k*math.tau/16;vertices.append(e+Vector((math.sin(polar)*math.cos(a),math.sin(polar)*math.sin(a),math.cos(polar)))*radius);groups.append(f'Forearm.{side}')
            for j in range(8):
                for k in range(16):faces.append((base+j*16+k,base+j*16+(k+1)%16,base+(j+1)*16+(k+1)%16,base+(j+1)*16+k));material_ids.append(0)
            mesh=bpy.data.meshes.new(obj.name+'_FixedSegments');mesh.from_pydata(vertices,[],faces);mesh.update();obj.data=mesh
            for m in materials:mesh.materials.append(m)
            obj.vertex_groups.clear()
            for name in set(groups):
                group=obj.vertex_groups.new(name=name);group.add([i for i,g in enumerate(groups) if g==name],1,'REPLACE')
            for p,mat in zip(mesh.polygons,material_ids):p.material_index=mat;p.use_smooth=True
