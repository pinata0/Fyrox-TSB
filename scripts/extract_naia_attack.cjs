// Evaluate the archived battle clip with Spine's original curves and IK.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js', 'utf8'));
const atlas = new spine.TextureAtlas(fs.readFileSync('artifacts/naia-attack/reference/skeleton.atlas', 'utf8'));
for(const page of atlas.pages) page.setTexture({getImage:()=>({width:page.width,height:page.height}),setFilters(){},setWraps(){}});
const bytes = fs.readFileSync('assets/references/naia-motion-reference.skel');
const data = new spine.SkeletonBinary(new spine.AtlasAttachmentLoader(atlas)).readSkeletonData(new Uint8Array(bytes));
const sk = new spine.Skeleton(data); sk.setSkin(data.findSkin('Normal') || data.skins[0]);
const anim = data.findAnimation('Attack1_1');
const round = n => +n.toFixed(6);
const wrap = n => ((n+180)%360+360)%360-180;
function pose(a,t) {sk.setToSetupPose(); a.apply(sk,0,Math.min(a.duration,t+1e-6),false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);sk.updateWorldTransform();}
pose(data.findAnimation('Idle'),0);
const hairNames = ['L','R'].flatMap(s => [0,1,2].map(i=>`Hair_Tail_${s}_${i}`));
const idleHair = Object.fromEntries(hairNames.map(n=>[n,sk.findBone(n).rotation]));
const samples=[];
for(let i=0;i<=100;i++) {
  pose(anim,anim.duration*i/100);
  const eye = s => ({attachment:sk.findSlot(`Eye_${s}`).attachment.name, scale:round(sk.findBone(`Eye_${s}`).scaleX)});
  samples.push({hair:hairNames.map(n=>round(wrap(sk.findBone(n).rotation-idleHair[n])*Math.PI/180)),eyes:[eye('L'),eye('R')],mouth:{attachment:sk.findSlot('Mouth_Set').attachment.name,scale:round(sk.findBone('Mouth').scaleX)}});
}
const attachments=anim.timelines.filter(t=>t.attachmentNames).map(t=>({slot:data.slots[t.slotIndex].name,times:Array.from(t.frames).map(round),names:t.attachmentNames}));
const record={clip:anim.name,duration:anim.duration,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),hairNames,attachments,samples};
fs.writeFileSync('assets/characters/attack/naia-detail.json',JSON.stringify(record));
const frames=[];
for(const phase of [0,.18,.30,.44,.54,.64,.78,1]) {
  pose(anim,anim.duration*phase);
  const meshes=[];
  for(const slot of sk.drawOrder) {
    const a=slot.attachment;
    if(!a || slot.color.a<.01 || !slot.bone.active) continue;
    let vertices=[],uvs=[],triangles=[];
    if(a instanceof spine.RegionAttachment) {
      vertices=new Array(8);a.computeWorldVertices(slot,vertices,0,2);uvs=Array.from(a.uvs);triangles=[0,1,2,2,3,0];
    } else if(a instanceof spine.MeshAttachment) {
      vertices=new Array(a.worldVerticesLength);a.computeWorldVertices(slot,0,vertices.length,vertices,0,2);uvs=Array.from(a.uvs);triangles=Array.from(a.triangles);
    } else continue;
    meshes.push({name:slot.data.name,attachment:a.name,vertices,uvs,triangles,alpha:slot.color.a*a.color.a});
  }
  frames.push({phase,meshes});
}
fs.writeFileSync('artifacts/naia-attack/reference/frames.json',JSON.stringify(frames));
console.log(JSON.stringify({duration:anim.duration,attachments,hairNames},null,2));
