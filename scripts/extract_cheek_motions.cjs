// Research-only Spine 4.1 sampler. No Spine runtime or textures ship in the game.
// Run after downloading the standing skeletons listed in research/sources.json.
const fs = require('fs');
const vm = require('vm');
const dir = 'artifacts/cheek-interaction/research';
vm.runInThisContext(fs.readFileSync(`${dir}/spine-core.js`, 'utf8'));
// Geometry/UVs are not needed when sampling skeletal transforms and attachment names.
spine.MeshAttachment.prototype.updateRegion = function () {};
spine.RegionAttachment.prototype.updateRegion = function () {};
const loader = {
  newRegionAttachment: (s,n,p) => new spine.RegionAttachment(n,p),
  newMeshAttachment: (s,n,p) => new spine.MeshAttachment(n,p),
  newBoundingBoxAttachment: (s,n) => new spine.BoundingBoxAttachment(n),
  newPathAttachment: (s,n) => new spine.PathAttachment(n),
  newPointAttachment: (s,n) => new spine.PointAttachment(n),
  newClippingAttachment: (s,n) => new spine.ClippingAttachment(n),
};
const sources = JSON.parse(fs.readFileSync(`${dir}/sources.json`, 'utf8'));
const round = x => +x.toFixed(5);
const wrap = x => ((x + 540) % 360 - 180) * Math.PI / 180;
const records = [], report = [];
// Eyelid compression and mouth shape fitted to the rendered Normal-skin contact
// sheet (original-motions.png). "Touch" means different expressions per hero:
// Naia/Aya/Renua have open startled eyes; Picora/Butter have tightly shut eyes.
// These are 3D approximations, not copies of the original attachment artwork.
const expressions = {
  vivi:[[.88,.88,.3],[0,0,.65]], naia:[[0,0,.2],[.88,.88,.6]],
  erpin:[[.88,.88,.3],[.9,.9,.7]], id:[[.65,.8,.2],[.85,.85,.6]],
  aya:[[0,0,.15],[.25,.25,.45]], picora:[[.94,.94,.3],[0,0,.8]],
  chloe:[[.8,.65,.25],[0,0,.65]], renua:[[0,0,.25],[.25,.25,-1]],
  mayo:[[.25,.25,-1],[.35,.35,-1]], bella:[[.85,.85,.25],[.25,.25,-1]],
  ui:[[.88,.5,.2],[.85,.65,.8]], butter:[[.94,.94,.45],[.94,.94,.65]],
  komi:[[.7,.7,.25],[0,0,.4]], hilde:[[.9,.9,.2],[0,0,.5]],
  shion:[[.9,.5,.25],[0,0,.6]],
};
for (const source of sources) {
  const data = new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(fs.readFileSync(`${dir}/${source.slug}.skel`)));
  const sk = new spine.Skeleton(data);
  sk.setSkin(data.findSkin('Normal') || data.findSkin('S1') || data.skins[0]);
  const find = (...names) => names.map(n => sk.findBone(n)).find(Boolean);
  const head = find('Head','S1_Head');
  const body = find('Body_2','S1_Body_2');
  const bones = [head,body,
    find('Upperarm_L','Upperarm_L_2','Uppearm_L','Arm_L_1','S1_Upperarm_L'),
    find('Forearm_L','Forearm_L_1','Arm_L_2','S1_Forearm_L'),
    find('Upperarm_R','Arm_R_1','S1_Upperarm_R'),
    find('Forearm_R','Forearm_R_1','Arm_R_2','S1_Forearm_R')];
  if (bones.some(b => !b)) throw new Error(`${source.slug}: retarget bone missing`);
  function pose(anim,t) {
    sk.setToSetupPose();
    anim.apply(sk,0,t,false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);
    sk.updateWorldTransform();
    return bones.map(b => [b.worldX,b.worldY,b.getWorldRotationX()]);
  }
  const idle = data.findAnimation('Idle_1');
  const base = pose(idle,0);
  const cheek = find('Character_Ball_Move');
  const faceWidth = Math.max(120, Math.abs(cheek.worldX-head.worldX)*2);
  const clips = {};
  const details = {};
  for (const name of ['Touch_Idle','Touch_End']) {
    const anim = data.findAnimation(name);
    if (!anim) throw new Error(`${source.slug}: ${name} missing`);
    const samples = [];
    const faces = [];
    const hands = [];
    const count = Math.ceil(anim.duration*30);
    for (let i=0;i<=count;i++) {
      const t = anim.duration*i/count;
      const p = pose(anim,t);
      // Screen-plane rotation offsets, plus displacement in units of face width.
      samples.push([...p.map((b,j) => wrap(b[2]-base[j][2])),
        (p[0][0]-base[0][0])/faceWidth,(p[0][1]-base[0][1])/faceWidth].map(round));
      const active = anim.timelines.filter(t=>t.attachmentNames).map(t=>{
        // Read the evaluated slot, so discrete attachment changes retain their timing.
        const slot=sk.slots[t.slotIndex];
        return {slot:slot.data.name,name:slot.attachment?.name||''};
      });
      const tears=['L','R'].reduce((bits,side,index)=>bits|(active.some(a=>/Tear/.test(a.slot)&&!/(Seba|Drone)/.test(a.slot)&&new RegExp(`Tear.*${side}`).test(a.slot)&&a.name)?1<<index:0),0);
      faces.push([...expressions[source.slug][name==='Touch_Idle'?0:1],tears]);
      const leftCheek=find('Ball_L','S1_F_Ball_L','S1_Ball_L','Ball_L_CT');
      const cx=leftCheek?(leftCheek.worldX+cheek.worldX)/2:head.worldX;
      const cy=leftCheek?(leftCheek.worldY+cheek.worldY)/2:head.worldY+faceWidth*.2;
      hands.push(['L','R'].flatMap(side=>{
        const candidates=sk.drawOrder.filter(s=>s.attachment&&new RegExp(`^(S1_)?Hand_${side}(_[123])?$`).test(s.data.name));
        const b=candidates[0]?.bone||find(`Hand_${side}`,`S1_Hand_${side}`,`Hand_${side}_1`,`Hand_${side}_2`)||bones[side==='L'?3:5];
        return [(b.worldX-cx)/faceWidth,(b.worldY-cy)/faceWidth].map(round);
      }));
    }
    const attachments = anim.timelines.filter(t => t.attachmentNames).map(t => ({
      slot:data.slots[t.slotIndex].name, frames:Array.from(t.frames).map(round), names:t.attachmentNames,
    })).filter(t => /Eye|Mouth|Tear|Angry|Shy|Blush|Heart/i.test(t.slot));
    clips[name === 'Touch_Idle' ? 'hold' : 'release'] = {duration:round(anim.duration),samples,faces,hands};
    details[name] = {duration:anim.duration,bones:bones.map(b=>b.data.name),attachments};
  }
  records.push({slug:source.slug,...clips});
  report.push({...source,skin:sk.skin.name,clips:details});
  console.log(source.slug,clips.hold.duration,clips.release.duration);
}
fs.mkdirSync('assets/characters/cheeks',{recursive:true});
fs.writeFileSync('assets/characters/cheeks/motions.json',JSON.stringify(records));
fs.writeFileSync(`${dir}/motion-report.json`,JSON.stringify(report,null,2));
