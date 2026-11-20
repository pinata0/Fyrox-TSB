// Archived Spine transforms, evaluated with their original curves and constraints.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js', 'utf8'));
spine.MeshAttachment.prototype.updateRegion = spine.RegionAttachment.prototype.updateRegion = function () {};
const loader = Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n =>
  [`new${n}Attachment`, (s,name,path) => new spine[`${n}Attachment`](name,path)]));
const bytes = fs.readFileSync('assets/references/id-motion-reference.skel');
const data = new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
const sk = new spine.Skeleton(data); sk.setSkin(data.findSkin('Normal'));
const names = ['LLEL','LLEL_OutBody_1','LLEL_OutBody_2','LLEL_OutBody_3','Nattah_Root','Body_2','Head',
  'Hair_B_1_1','Hair_B_1_2','Hair_B_1_3','Hair_B_2_1','Hair_B_2_2','Hair_B_2_3','Hair_L_1_1','Hair_L_1_2',
  'LLEL_Ring1','LLEL_Ring2'];
const round=x=>+x.toFixed(6), wrap=x=>((x+180)%360+360)%360-180;
function pose(anim,t) {
  sk.setToSetupPose(); anim.apply(sk,0,t+1e-6,false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn); sk.updateWorldTransform();
  return Object.fromEntries(names.map(n=>{const b=sk.findBone(n);return [n,[b.worldX,b.worldY,b.getWorldRotationX(),b.x,b.y,b.rotation]];}));
}
const idle=pose(data.findAnimation('Idle'),0), clips=[];
for(const name of ['Skill1_1','Ultimate1_1']) {
  const anim=data.findAnimation(name), count=Math.round(anim.duration*30), samples=[];
  for(let frame=0;frame<=count;frame++) {
    const p=pose(anim,frame/30);
    const delta=(n,local=false)=>[round((p[n][local?3:0]-idle[n][local?3:0])/650),round((p[n][local?4:1]-idle[n][local?4:1])/650),round(wrap(p[n][local?5:2]-idle[n][local?5:2])*Math.PI/180)];
    samples.push({mount:delta('LLEL'),panels:[1,2,3].map(n=>delta('LLEL_OutBody_'+n,true)),nata:delta('Nattah_Root'),
      torso:delta('Body_2',true)[2],head:delta('Head',true)[2],hair:names.filter(n=>n.startsWith('Hair')).map(n=>delta(n,true)[2]),
      eyes:['R','L'].map(side=>sk.findSlot('Eye_'+side).attachment?.name.includes('Close')?0:1),
      cover:!!sk.findSlot('LLEL_Galss').attachment,
      rings:[1,2].map(n=>{const b=sk.findBone('LLEL_Ring'+n);return [round(b.rotation*Math.PI/180),round(Math.abs(b.scaleX))];})});
  }
  clips.push({clip:name,duration:count/30,samples});
}
const q=clips[1];
const release=q.samples.findIndex(s=>s.eyes.every(Boolean))/30;
fs.writeFileSync('assets/characters/skill/id.json',JSON.stringify({sha256:crypto.createHash('sha256').update(bytes).digest('hex'),release,clips}));
console.log(clips.map(c=>({clip:c.clip,frames:c.samples.length})),{release});
