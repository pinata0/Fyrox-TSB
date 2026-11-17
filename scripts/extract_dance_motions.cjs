// Sample the original standing Dance_1 tracks; runtime/atlas assets stay research-only.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
const dir = 'artifacts/cheek-interaction/research';
vm.runInThisContext(fs.readFileSync(`${dir}/spine-core.js`, 'utf8'));
spine.MeshAttachment.prototype.updateRegion = function() {};
spine.RegionAttachment.prototype.updateRegion = function() {};
const loader = Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(type =>
  [`new${type}Attachment`, (s,n,p) => new spine[`${type}Attachment`](n,p)]));
const sources = JSON.parse(fs.readFileSync(`${dir}/sources.json`));
const slugs = ['aya','picora','chloe','renua','hilde'];
const round = v => +v.toFixed(6);
const records = [];
for (const slug of slugs) {
  const bytes = fs.readFileSync(`${dir}/${slug}.skel`);
  const data = new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
  const sk = new spine.Skeleton(data);
  sk.setSkin(data.findSkin('Normal'));
  const anim = data.findAnimation('Dance_1');
  const find = (...names) => names.map(n=>sk.findBone(n)).find(Boolean);
  const at = b => [b.worldX,b.worldY].map(round);
  const samples = [], mapping = {};
  const frameCount = 120;
  for(let f=0; f<=frameCount; f++) {
    sk.setToSetupPose();
    // Explicitly duplicate the first pose at the baked loop boundary.
    anim.apply(sk,0,f===frameCount?0:anim.duration*f/frameCount,false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);
    sk.updateWorldTransform();
    const pelvis = find('Pelvis_VCT','Plevis_VCT');
    const head = find('Head');
    const p = {pelvis:at(pelvis),head:at(head),headAngle:round(head.getWorldRotationX()),arms:{},legs:{}};
    for(const side of ['L','R']) {
      const upper = find(`Upperarm_${side}`,`Uppearm_${side}`,`Arm_${side}_1`);
      let elbow = find(`Forearm_${side}`,`Arm_${side}_2`);
      let hand = find(`Hand_${side}`,`Hand_${side}_1`);
      // Renua switches hand/forearm attachments between bent-up and bent-down chains.
      const activeHands = sk.drawOrder.filter(s=>s.attachment && new RegExp(`^Hand_${side}(_[123])?$`).test(s.data.name));
      if(slug==='renua' && activeHands.length) {
        hand = activeHands[0].bone;
        const lineage=[];for(let b=hand;b;b=b.parent)lineage.push(b);
        elbow=lineage.find(b=>new RegExp(`^Arm_${side}_2(?:_U|_Down)?$`).test(b.data.name))||elbow;
      }
      // Aya swaps her free arm to its raised chain in Dance_1.
      if(slug==='aya' && side==='L') {
        const up=sk.drawOrder.some(s=>s.attachment && /(?:Upperarm|Forearm|Hand)_L.*Up/.test(s.data.name));
        if(up){elbow=find('Forearm_L_Up');hand=find('Hand_L_Up');}
      }
      const thigh = find(`Thigh_${side}`,`Leg_${side}_1`);
      let calf = find(`Calf_${side}`,`Leg_${side}_2`, `Calf_${side}_1`);
      let foot = find(`Foot_${side}`,`Foot_${side}_1`,`Leg_${side}_IK`);
      if(slug==='picora' && side==='L') {
        // The lifted shin (Calf_L_1) is a separate visible attachment chain.
        calf=find('Calf_L_1');foot=find('Foot_L_1');
      }
      if([upper,elbow,hand,thigh,calf,foot].some(b=>!b))throw Error(`${slug}/${side}: incomplete joint mapping`);
      p.arms[side]=[upper,elbow,hand].map(at);
      p.legs[side]=[thigh,calf,foot].map(at);
      mapping[side]={arm:[upper,elbow,hand].map(b=>b.data.name),leg:[thigh,calf,foot].map(b=>b.data.name)};
    }
    samples.push(p);
  }
  records.push({slug,clip:anim.name,duration:round(anim.duration),source:sources.find(s=>s.slug===slug),
    sha256:crypto.createHash('sha256').update(bytes).digest('hex'),mapping,samples});
  console.log(slug,anim.name,anim.duration.toFixed(3));
}
fs.mkdirSync('artifacts/dance-research',{recursive:true});
fs.writeFileSync('artifacts/dance-research/sampled-dances.json',JSON.stringify(records));
