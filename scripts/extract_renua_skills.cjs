// Archived Spine curves, IK, cloth, halo and summoned constructs at native 30 Hz.
const fs=require('fs'),vm=require('vm'),crypto=require('crypto');
const out='artifacts/renua-skills/reference';fs.mkdirSync(out,{recursive:true});
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js','utf8'));
const atlas=new spine.TextureAtlas(fs.readFileSync(`${out}/skeleton.atlas`,'utf8'));
for(const page of atlas.pages)page.setTexture({getImage:()=>({width:page.width,height:page.height}),setFilters(){},setWraps(){}});
const bytes=fs.readFileSync('assets/references/renua-motion-reference.skel');
const data=new spine.SkeletonBinary(new spine.AtlasAttachmentLoader(atlas)).readSkeletonData(new Uint8Array(bytes));
const sk=new spine.Skeleton(data);sk.setSkin(data.findSkin('Normal')||data.skins[0]);
const mapping=JSON.parse(fs.readFileSync('artifacts/basic-attacks/research/motion-report.json')).find(r=>r.slug==='renua').bones;
const bones=Object.fromEntries(Object.entries(mapping).map(([k,n])=>[k,sk.findBone(n)]));
const parents={Hips:null,Torso:'Hips',Head:'Torso'};
for(const s of ['L','R'])Object.assign(parents,{[`UpperArm.${s}`]:'Torso',[`Forearm.${s}`]:`UpperArm.${s}`,[`Hand.${s}`]:`Forearm.${s}`,[`Thigh.${s}`]:'Hips',[`Shin.${s}`]:`Thigh.${s}`,[`Foot.${s}`]:`Shin.${s}`});
const detailNames=['Halo','Halo1','Halo2','Coat_body_back','Coat_body_back2','Coat_body_back3','Body_7','Body_8','Body_12','Body_13','Hair_Back_L','Hair_Back_L1','Hair_Back_R','Hair_Back_R1'];
const constructSlots=sk.slots.filter(s=>/^Weapon/.test(s.data.name));
const round=x=>+x.toFixed(6),wrap=x=>((x+180)%360+360)%360-180;
function pose(anim,t){sk.setToSetupPose();anim.apply(sk,0,Math.min(anim.duration,t+1e-6),false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);sk.updateWorldTransform();return Object.fromEntries(Object.entries(bones).map(([k,b])=>[k,[b.worldX,b.worldY,b.getWorldRotationX()]]));}
function meshes(){return sk.drawOrder.flatMap(slot=>{
 const a=slot.attachment;if(!a||slot.color.a<.01||!slot.bone.active)return [];
 let vertices,uvs,triangles;
 if(a instanceof spine.RegionAttachment){vertices=new Array(8);a.computeWorldVertices(slot,vertices,0,2);uvs=Array.from(a.uvs);triangles=[0,1,2,2,3,0];}
 else if(a instanceof spine.MeshAttachment){vertices=new Array(a.worldVerticesLength);a.computeWorldVertices(slot,0,vertices.length,vertices,0,2);uvs=Array.from(a.uvs);triangles=Array.from(a.triangles);}else return [];
 return [{name:slot.data.name,attachment:a.name,vertices,uvs,triangles,alpha:slot.color.a*a.color.a}];
});}
const idle=pose(data.findAnimation('Idle'),0),height=idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]);
const rest=detailNames.map(n=>sk.findBone(n).rotation),clips=[],frames=[];
for(const name of ['Idle','Attack1_1','Attack2_1','Skill1_1','Ultimate1_1']){
 const anim=data.findAnimation(name),count=Math.round(anim.duration*30),samples=[];let previous={},details=rest.map(()=>0);
 for(let i=0;i<=count;i++){
  const p=pose(anim,i/30),rotations={};
  for(const k of Object.keys(bones)){const parent=parents[k];let a=wrap(p[k][2]-idle[k][2]-(parent?p[parent][2]-idle[parent][2]:0))*Math.PI/180;if(previous[k]!==undefined)a=previous[k]+wrap((a-previous[k])*180/Math.PI)*Math.PI/180;rotations[k]=round(a);}previous=rotations;
  details=detailNames.map((n,j)=>{let a=wrap(sk.findBone(n).rotation-rest[j])*Math.PI/180;return round(details[j]+wrap((a-details[j])*180/Math.PI)*Math.PI/180);});
  samples.push({rotations,root:[round((p.Hips[0]-idle.Hips[0])/height),round((p.Hips[1]-idle.Hips[1])/height)],
   hands:['L','R'].map(s=>{const k=`Hand.${s}`;return [round((p[k][0]-p.Torso[0]-idle[k][0]+idle.Torso[0])/height),round((p[k][1]-p.Torso[1]-idle[k][1]+idle.Torso[1])/height),round(wrap(p[k][2]-idle[k][2])*Math.PI/180)];}),details,
   constructs:constructSlots.map(s=>{const b=s.bone;return [round((b.worldX-p.Hips[0])/height),round((b.worldY-p.Hips[1])/height),round(b.getWorldRotationX()*Math.PI/180),round(Math.abs(b.getWorldScaleX())),round(Math.abs(b.getWorldScaleY())),round(s.attachment?s.color.a*s.attachment.color.a:0)];})});
 }
 const events=anim.timelines.filter(t=>t.events).flatMap(t=>t.events.filter(e=>e.data.name==='Event').map(e=>round(e.time)));
 clips.push({clip:name,duration:count/30,events,samples});
 if(name!=='Idle')for(const phase of [0,.12,.24,.38,.48,.61,.76,.90]){pose(anim,phase*anim.duration);frames.push({clip:name,phase,meshes:meshes()});}
}
fs.writeFileSync('assets/characters/skill/renua.json',JSON.stringify({sha256:crypto.createHash('sha256').update(bytes).digest('hex'),detailNames,constructSlots:constructSlots.map(s=>({slot:s.data.name,bone:s.bone.data.name})),clips}));
fs.writeFileSync(`${out}/frames.json`,JSON.stringify(frames));
fs.writeFileSync(`${out}/source.json`,JSON.stringify({mapping,height,detailNames,constructSlots:constructSlots.map(s=>({slot:s.data.name,bone:s.bone.data.name})),clips:clips.map(({samples,...c})=>c)},null,2));
console.log(clips.map(c=>({clip:c.clip,frames:c.samples.length,events:c.events})));
