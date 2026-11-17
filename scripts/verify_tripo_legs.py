"""Independent lower-limb fitting and animation checks used by the roster verifier."""
import numpy as np
from import_tripo_butter import components


def section(rest, edges, ids, z):
    membership=np.zeros(len(rest),dtype=bool);membership[list(ids)]=True
    pair=edges[membership[edges].all(axis=1)]
    a=rest[pair[:,0]];b=rest[pair[:,1]]
    valid=(np.minimum(a[:,2],b[:,2])<=z)&(np.maximum(a[:,2],b[:,2])>=z)&(np.abs(a[:,2]-b[:,2])>1e-8)
    a=a[valid];b=b[valid]
    assert len(a)>4,('missing continuous leg cross-section',z,len(ids))
    ring=a+(b-a)*((z-a[:,2])/(b[:,2]-a[:,2]))[:,None]
    lo=ring[:,:2].min(axis=0);hi=ring[:,:2].max(axis=0)
    return (lo+hi)*.5,(hi-lo)*.5


def prepare(slug,mesh,rig,rest,body_ids,cloth_ids):
    attribute=mesh.data.attributes.get('leg_part')
    assert attribute is not None,(slug,'missing leg fitting regions')
    tags=np.array([v.value for v in attribute.data])
    edges=np.array([e.vertices[:] for e in mesh.data.edges])
    report={};soles={}
    native_shorts=set()
    if slug=='renua':
        for ids in components(mesh.data):
            z=rest[ids,2]/1.85
            if .30<z.min()<.32 and .41<z.max()<.43 and all(i in cloth_ids for i in ids):
                native_shorts.update(ids)
        assert len(native_shorts)>500,'missing fitted shorts surface'
    for side,sign,boot_tag,stock_tag,sandal_tag in [('R',1,3,1,5),('L',-1,4,2,6)]:
        footwear=set(np.where(tags==(sandal_tag if slug=='naia' else boot_tag))[0])
        assert len(footwear)>50,(slug,side,'missing footwear')
        skin={i for i in body_ids if rest[i,0]*sign>.002*1.85 and abs(rest[i,0])<.15*1.85}
        measurements={}
        if slug=='naia':
            assert not any(rest[i,2]<.28*1.85 for i in cloth_ids),'grey mannequin legs returned'
            for z in [.07,.15,.25]:
                _,radius=section(rest,edges,skin,z*1.85)
                assert min(radius)>.02,(slug,side,'native leg was cut away',z)
            measurements['continuous_bare_leg_sections']=3
        else:
            stocking=set(np.where(tags==stock_tag)[0]) if slug=='erpin' else footwear
            z=(.286 if slug=='erpin' else .281)*1.85
            center,radius=section(rest,edges,stocking,z)
            skin_center,skin_radius=section(rest,edges,skin,z)
            offset=float(np.linalg.norm(center-skin_center))
            clearance=float(np.min(radius-skin_radius-np.abs(center-skin_center)))
            assert offset<.015,(slug,side,'garment opening misses the leg',offset)
            assert clearance>.001,(slug,side,'leg does not fit through opening',clearance)
            measurements.update(opening_center_offset_m=offset,opening_clearance_m=clearance)
            if slug=='erpin':
                a,ar=section(rest,edges,stocking,.085*1.85)
                b,br=section(rest,edges,footwear,.085*1.85)
                seam_offset=float(np.linalg.norm(a-b))
                assert seam_offset<.012,(slug,side,'stocking misses boot',seam_offset)
                overlap=float(rest[list(footwear),2].max()-rest[list(stocking),2].min())
                assert overlap>.02,(slug,side,'no stocking/boot overlap',overlap)
                measurements.update(ankle_center_offset_m=seam_offset,stocking_boot_overlap_m=overlap)
            else:
                shorts={i for i in native_shorts if rest[i,0]*sign>0}
                # The original leg ends on a slanted cap. Compare axes below
                # that cap, then require overlap with the independent shorts.
                a,_=section(rest,edges,shorts,.33*1.85)
                b,_=section(rest,edges,skin,.31*1.85)
                assert np.linalg.norm(a-b)<.015,(slug,side,'shorts miss the thigh',a,b)
                lower_skin=[i for i in skin if rest[i,2]<.34*1.85]
                overlap=float(rest[lower_skin,2].max()-rest[list(shorts),2].min())
                assert overlap>.015,(slug,side,'gap below shorts',overlap)
                measurements['shorts_thigh_overlap_m']=overlap
            for i in footwear:
                assert all(mesh.vertex_groups[g.group].name in [f'Thigh.{side}',f'Shin.{side}',f'Foot.{side}']
                           for g in mesh.data.vertices[int(i)].groups),(slug,side,'boot assigned outside its leg')
        foot=f'Foot.{side}';index=mesh.vertex_groups[foot].index
        ids=np.array(sorted(i for i in footwear if rest[i,2]<rig.data.bones[foot].head_local.z+.004*1.85))
        assert len(ids)>20
        assert all(next((g.weight for g in mesh.data.vertices[int(i)].groups if g.group==index),0)>.99999
                   for i in ids),(slug,side,'shoe sole bends away from foot')
        soles[side]=(ids,np.column_stack((rest[ids],np.ones(len(ids)))),foot)
        report[side]={**measurements,'footwear_vertices':len(footwear),'sole_vertices':len(ids),'max_sole_drift_m':0.0}
    return report,soles


def evaluate(slug,rig,points,report,soles,frame):
    for side,(ids,rest,bone) in soles.items():
        transform=np.array(rig.pose.bones[bone].matrix@rig.data.bones[bone].matrix_local.inverted())
        expected=(rest@transform.T)[:,:3]
        drift=float(np.max(np.abs(points[ids]-expected)))
        report[side]['max_sole_drift_m']=max(report[side]['max_sole_drift_m'],drift)
        assert drift<.00001,(slug,side,frame,'footwear detached',drift)
