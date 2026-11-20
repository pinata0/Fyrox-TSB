// Sample the archived battle skeleton with its original constraints at 30 Hz.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js', 'utf8'));
spine.MeshAttachment.prototype.updateRegion = spine.RegionAttachment.prototype.updateRegion = function () {};
const loader = Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n =>
  [`new${n}Attachment`, (s,name,path) => new spine[`${n}Attachment`](name,path)]));
const bytes = fs.readFileSync('assets/references/shion-motion-reference.skel');
const data = new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
const sk = new spine.Skeleton(data); sk.setSkin(data.findSkin('Normal'));
const mapping = JSON.parse(fs.readFileSync('artifacts/basic-attacks/research/motion-report.json')).find(r => r.slug === 'shion').bones;
const parents = {Hips:null,Torso:'Hips',Head:'Torso'};
for (const s of ['L','R']) Object.assign(parents, {[`UpperArm.${s}`]:'Torso',[`Forearm.${s}`]:`UpperArm.${s}`,[`Hand.${s}`]:`Forearm.${s}`,[`Thigh.${s}`]:'Hips',[`Shin.${s}`]:`Thigh.${s}`,[`Foot.${s}`]:`Shin.${s}`});
const round = x => +x.toFixed(6), wrap = x => ((x+180)%360+360)%360-180;
function pose(anim,t) {
  sk.setToSetupPose(); anim.apply(sk,0,Math.min(anim.duration,t+1e-6),false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn); sk.updateWorldTransform();
  return Object.fromEntries(Object.entries(mapping).map(([k,n])=>{const b=sk.findBone(n);return [k,[b.worldX,b.worldY,b.getWorldRotationX()]];}));
}
const idle=pose(data.findAnimation('Idle'),0), height=idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]);
const clips=[];
for(const name of ['Skill1_1','Skill1_1_Loop','Skill1_2','Ultimate1_1']) {
  const anim=data.findAnimation(name), count=Math.round(anim.duration*30), samples=[];
  for(let i=0;i<=count;i++) {
    const p=pose(anim,i/30), rotations={};
    for(const k of Object.keys(mapping)) {const parent=parents[k];rotations[k]=round(wrap(p[k][2]-idle[k][2]-(parent?p[parent][2]-idle[parent][2]:0))*Math.PI/180);}
    samples.push({rotations,root:[round((p.Hips[0]-idle.Hips[0])/height),round((p.Hips[1]-idle.Hips[1])/height)],
      hands:['L','R'].map(s=>{const k=`Hand.${s}`;return [round((p[k][0]-p.Torso[0]-idle[k][0]+idle.Torso[0])/height),round((p[k][1]-p.Torso[1]-idle[k][1]+idle.Torso[1])/height),round(wrap(p[k][2]-idle[k][2])*Math.PI/180)];})});
  }
  const events=anim.timelines.filter(t=>t.events).flatMap(t=>t.events.map(e=>({name:e.data.name,time:round(e.time)})));
  clips.push({clip:name,duration:count/30,events,samples});
}
fs.writeFileSync('assets/characters/skill/shion.json',JSON.stringify({sha256:crypto.createHash('sha256').update(bytes).digest('hex'),mapping,clips}));
console.log(JSON.stringify(clips.map(({samples,...clip})=>({...clip,frames:samples.length})),null,2));
