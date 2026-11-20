// Evaluate the archived battle skeleton offline, using its original IK/curves.
const fs=require('fs'),vm=require('vm'),crypto=require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js','utf8'));
spine.MeshAttachment.prototype.updateRegion=spine.RegionAttachment.prototype.updateRegion=function(){};
const loader=Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n=>[`new${n}Attachment`,(s,name,path)=>new spine[`${n}Attachment`](name,path)]));
const bytes=fs.readFileSync('assets/references/erpin-motion-reference.skel');
const data=new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
const sk=new spine.Skeleton(data);sk.setSkin(data.findSkin('Normal')||data.skins[0]);
const mapping=JSON.parse(fs.readFileSync('artifacts/basic-attacks/research/motion-report.json')).find(r=>r.slug==='erpin').bones;
const bones=Object.fromEntries(Object.entries(mapping).map(([k,n])=>[k,sk.findBone(n)]));
const parents={Hips:null,Torso:'Hips',Head:'Torso'};
for(const s of ['L','R'])Object.assign(parents,{[`UpperArm.${s}`]:'Torso',[`Forearm.${s}`]:`UpperArm.${s}`,[`Hand.${s}`]:`Forearm.${s}`,[`Thigh.${s}`]:'Hips',[`Shin.${s}`]:`Thigh.${s}`,[`Foot.${s}`]:`Shin.${s}`});
const round=x=>+x.toFixed(6),wrap=x=>((x+180)%360+360)%360-180;
function pose(a,t){sk.setToSetupPose();a.apply(sk,0,Math.min(a.duration,t+1e-6),false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);sk.updateWorldTransform();return Object.fromEntries(Object.entries(bones).map(([k,b])=>[k,[b.worldX,b.worldY,b.getWorldRotationX()]]));}
const idle=pose(data.findAnimation('Idle'),0),height=idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]);
const records=[];
for(const name of ['Skill1_1','Ultimate1_1','Ultimate1_2_Loop','Ultimate1_3']){
 const anim=data.animations.find(a=>a.name.toLowerCase()===name.toLowerCase()),count=Math.round(anim.duration*30),samples=[];let previous={};
 for(let i=0;i<=count;i++){
  const p=pose(anim,anim.duration*i/count),rotations={};
  for(const k of Object.keys(bones)){const parent=parents[k];let a=wrap(p[k][2]-idle[k][2]-(parent?p[parent][2]-idle[parent][2]:0))*Math.PI/180;if(previous[k]!==undefined)a=previous[k]+wrap((a-previous[k])*180/Math.PI)*Math.PI/180;rotations[k]=round(a);}
  previous=rotations;
  const hands=['L','R'].map(s=>{const k=`Hand.${s}`;return [round((p[k][0]-p.Torso[0]-idle[k][0]+idle.Torso[0])/height),round((p[k][1]-p.Torso[1]-idle[k][1]+idle.Torso[1])/height),rotations[k]+rotations[`Forearm.${s}`]+rotations[`UpperArm.${s}`]];});
  samples.push({rotations,root:[round((p.Hips[0]-idle.Hips[0])/height),round((p.Hips[1]-idle.Hips[1])/height)],hands});
 }
 const events=anim.timelines.filter(t=>t.events).flatMap(t=>t.events.map(e=>({name:e.data.name,time:round(e.time)})));
 const attachments=anim.timelines.filter(t=>t.attachmentNames).map(t=>({slot:data.slots[t.slotIndex].name,times:Array.from(t.frames).map(round),names:t.attachmentNames}));
 records.push({clip:name,duration:count/30,events,attachments,samples});
}
fs.mkdirSync('assets/characters/skill',{recursive:true});fs.mkdirSync('artifacts/erpin-skills/reference',{recursive:true});
fs.writeFileSync('assets/characters/skill/erpin.json',JSON.stringify(records));
fs.writeFileSync('artifacts/erpin-skills/reference/source.json',JSON.stringify({sha256:crypto.createHash('sha256').update(bytes).digest('hex'),mapping,clips:records.map(({samples,...c})=>c)},null,2));
console.log(JSON.stringify(records.map(({samples,...c})=>c),null,2));
