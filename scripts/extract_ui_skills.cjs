// Evaluate the archived Spine IK, unwrapped clover spin, ponytail and rider tracks.
const fs=require('fs'),vm=require('vm'),crypto=require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js','utf8'));
spine.MeshAttachment.prototype.updateRegion=spine.RegionAttachment.prototype.updateRegion=function(){};
const loader=Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n=>[`new${n}Attachment`,(s,name,path)=>new spine[`${n}Attachment`](name,path)]));
const bytes=fs.readFileSync('assets/references/ui-motion-reference.skel');
const data=new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
const sk=new spine.Skeleton(data);sk.setSkin(data.findSkin('Normal')||data.skins[0]);
const mapping=JSON.parse(fs.readFileSync('artifacts/basic-attacks/research/motion-report.json')).find(r=>r.slug==='ui').bones;
Object.assign(mapping,{Hair:'Hair_B_2_1',HairTip:'Hair_B_2_3',Mount:'Eru_Body',Clover:'Weapon_Leaf_0',Leaf:'Weapon_Leaf_1',Weapon:'Weapon_1'});
const bones=Object.fromEntries(Object.entries(mapping).map(([k,n])=>[k,sk.findBone(n)]));
const parents={Hips:null,Torso:'Hips',Head:'Torso',Hair:'Head',HairTip:'Hair',Mount:null,Clover:'Weapon',Leaf:'Clover',Weapon:'Hand.R'};
for(const s of ['L','R'])Object.assign(parents,{[`UpperArm.${s}`]:'Torso',[`Forearm.${s}`]:`UpperArm.${s}`,[`Hand.${s}`]:`Forearm.${s}`,[`Thigh.${s}`]:'Hips',[`Shin.${s}`]:`Thigh.${s}`,[`Foot.${s}`]:`Shin.${s}`});
const round=x=>+x.toFixed(6),wrap=x=>((x+180)%360+360)%360-180;
function pose(anim,t){sk.setToSetupPose();anim.apply(sk,0,Math.min(anim.duration,t+1e-6),false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);sk.updateWorldTransform();return Object.fromEntries(Object.entries(bones).map(([k,b])=>[k,[b.worldX,b.worldY,b.getWorldRotationX(),b.getWorldScaleX(),b.getWorldScaleY()]]));}
const idle=pose(data.findAnimation('Idle'),0),height=idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]);
const clips=[];
for(const name of ['Skill1_1','Ultimate1_1']){
 const anim=data.findAnimation(name),count=Math.round(anim.duration*30),samples=[];let previous={};
 for(let i=0;i<=count;i++){
  const p=pose(anim,i/30),rotations={};
  for(const k of Object.keys(bones)){const parent=parents[k];let a=wrap(p[k][2]-idle[k][2]-(parent?p[parent][2]-idle[parent][2]:0))*Math.PI/180;if(previous[k]!==undefined)a=previous[k]+wrap((a-previous[k])*180/Math.PI)*Math.PI/180;rotations[k]=round(a);}previous=rotations;
  samples.push({rotations,root:[round((p.Hips[0]-idle.Hips[0])/height),round((p.Hips[1]-idle.Hips[1])/height)],mount:[round((p.Mount[0]-idle.Mount[0])/height),round((p.Mount[1]-idle.Mount[1])/height)],clover:[rotations.Clover,round(p.Leaf[3]/idle.Leaf[3]),round(p.Leaf[4]/idle.Leaf[4])],hands:['L','R'].map(s=>{const k=`Hand.${s}`;return [round((p[k][0]-p.Torso[0]-idle[k][0]+idle.Torso[0])/height),round((p[k][1]-p.Torso[1]-idle[k][1]+idle.Torso[1])/height),round(wrap(p[k][2]-idle[k][2])*Math.PI/180)];})});
 }
 const events=anim.timelines.filter(t=>t.events).flatMap(t=>t.events.filter(e=>e.data.name==='Event').map(e=>round(e.time)));
 clips.push({clip:name,duration:count/30,events,samples});
}
fs.mkdirSync('assets/characters/skill',{recursive:true});fs.mkdirSync('artifacts/ui-skills/reference',{recursive:true});
fs.writeFileSync('assets/characters/skill/ui.json',JSON.stringify({sha256:crypto.createHash('sha256').update(bytes).digest('hex'),clips}));
fs.writeFileSync('artifacts/ui-skills/reference/source.json',JSON.stringify({mapping,height,clips:clips.map(({samples,...c})=>c)},null,2));
console.log(clips.map(c=>({clip:c.clip,frames:c.samples.length,events:c.events,spin:c.samples.map(s=>s.clover[0]).reduce((a,b)=>[Math.min(a[0],b),Math.max(a[1],b)],[0,0]),scale:c.samples.map(s=>s.clover[1]).reduce((a,b)=>[Math.min(a[0],b),Math.max(a[1],b)],[1,1])})));
