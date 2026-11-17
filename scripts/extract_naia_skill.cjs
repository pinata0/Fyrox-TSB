// Sample the four archived Naia E clips with Spine's curves and IK, offline.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js', 'utf8'));
spine.MeshAttachment.prototype.updateRegion = spine.RegionAttachment.prototype.updateRegion = function () {};
const loader = Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n =>
  [`new${n}Attachment`, (s,name,path) => new spine[`${n}Attachment`](name,path)]));
const bytes = fs.readFileSync('assets/references/naia-motion-reference.skel');
const data = new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
const sk = new spine.Skeleton(data); sk.setSkin(data.findSkin('Normal') || data.skins[0]);
const mapping = {Hips:'Body_1',Torso:'Body_2',Head:'Head'};
for (const s of ['L','R']) for (const [key, source] of Object.entries({UpperArm:'UpArm',Forearm:'Arm',Hand:'Hand',Thigh:'UpLeg',Shin:'Leg',Foot:'Foot'})) mapping[`${key}.${s}`]=`${source}_${s}`;
const parents={Hips:null,Torso:'Hips',Head:'Torso'};
for (const s of ['L','R']) Object.assign(parents,{[`UpperArm.${s}`]:'Torso',[`Forearm.${s}`]:`UpperArm.${s}`,[`Hand.${s}`]:`Forearm.${s}`,[`Thigh.${s}`]:'Hips',[`Shin.${s}`]:`Thigh.${s}`,[`Foot.${s}`]:`Shin.${s}`});
const bones=Object.fromEntries(Object.entries(mapping).map(([key,n])=>[key,sk.findBone(n)]));
for (const [key,b] of Object.entries(bones)) if(!b) throw Error(key);
const hairNames=['L','R'].flatMap(s=>[0,1,2].map(i=>`Hair_Tail_${s}_${i}`));
const round=x=>+x.toFixed(6), wrap=x=>((x+180)%360+360)%360-180;
function pose(anim,t){sk.setToSetupPose();anim.apply(sk,0,Math.min(anim.duration,t+1e-6),false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);sk.updateWorldTransform();return Object.fromEntries(Object.entries(bones).map(([k,b])=>[k,[b.worldX,b.worldY,b.getWorldRotationX()]]));}
const idle=pose(data.findAnimation('Idle'),0), idleHair=hairNames.map(n=>sk.findBone(n).rotation);
const height=Math.max(100,idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]));
const records=[];
for (let clip=1;clip<=4;clip++) {
  const anim=data.findAnimation(`Skill1_${clip}`); if(!anim)throw Error(`Missing Skill1_${clip}`);
  const count=Math.round(anim.duration*30), samples=[];
  for(let i=0;i<=count;i++) {
    const p=pose(anim,anim.duration*i/count), rotations={};
    for(const k of Object.keys(bones)) {const parent=parents[k];rotations[k]=round(wrap(p[k][2]-idle[k][2]-(parent?p[parent][2]-idle[parent][2]:0))*Math.PI/180);}
    samples.push({rotations,root:[round((p.Hips[0]-idle.Hips[0])/height),round((p.Hips[1]-idle.Hips[1])/height)],
      hands:['L','R'].map(s=>{const k=`Hand.${s}`;return [round((p[k][0]-p.Torso[0]-idle[k][0]+idle.Torso[0])/height),round((p[k][1]-p.Torso[1]-idle[k][1]+idle.Torso[1])/height),round(wrap(p[k][2]-idle[k][2])*Math.PI/180)];}),
      hair:hairNames.map((n,j)=>round(wrap(sk.findBone(n).rotation-idleHair[j])*Math.PI/180))});
  }
  records.push({clip:anim.name,duration:count/30,samples});
}
fs.mkdirSync('assets/characters/skill',{recursive:true});
fs.writeFileSync('assets/characters/skill/naia.json',JSON.stringify({sha256:crypto.createHash('sha256').update(bytes).digest('hex'),clips:records}));
console.log(records.map(r=>({clip:r.clip,duration:r.duration,frames:r.samples.length})));
