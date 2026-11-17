// Offline sampler of the archived battle Spawn tracks. No Spine runtime ships in game.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js', 'utf8'));
spine.MeshAttachment.prototype.updateRegion = function () {};
spine.RegionAttachment.prototype.updateRegion = function () {};
const loader = Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n =>
  [`new${n}Attachment`, (s,name,path) => new spine[`${n}Attachment`](name,path)]));
const sources = JSON.parse(fs.readFileSync('assets/references/manifest.json', 'utf8'));
const mapping = {
  Hips:['Body_1','body_1'], Torso:['Body_2','body_2'], Head:['Head','head'],
  'UpperArm.L':['UpArm_L','arm_1_L'], 'Forearm.L':['Arm_L','arm_2_L'], 'Hand.L':['Hand_L','hand_L'],
  'UpperArm.R':['UpArm_R','arm_1_R'], 'Forearm.R':['Arm_R','arm_2_R'], 'Hand.R':['Hand_R','hand_R','hand_1_R'],
  'Thigh.L':['UpLeg_L','leg_1_L'], 'Shin.L':['Leg_L','leg_2_L'], 'Foot.L':['Foot_L','foot_L','Leg_L','leg_2_L'],
  'Thigh.R':['UpLeg_R','leg_1_R'], 'Shin.R':['Leg_R','leg_2_R'], 'Foot.R':['Foot_R','foot_R','Leg_R','leg_2_R'],
};
const parents = {Hips:null,Torso:'Hips',Head:'Torso'};
for (const s of ['L','R']) Object.assign(parents, {
  [`UpperArm.${s}`]:'Torso',[`Forearm.${s}`]:`UpperArm.${s}`,[`Hand.${s}`]:`Forearm.${s}`,
  [`Thigh.${s}`]:'Hips',[`Shin.${s}`]:`Thigh.${s}`,[`Foot.${s}`]:`Shin.${s}`,
});
const round = x => +x.toFixed(6), wrap = x => ((x+180)%360+360)%360-180;
const records = [];
for (const source of sources) {
  const bytes = fs.readFileSync(`assets/references/${source.slug}-motion-reference.skel`);
  const data = new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
  const sk = new spine.Skeleton(data);
  sk.setSkin(data.findSkin('Normal') || data.skins[0]);
  const bones = Object.fromEntries(Object.entries(mapping).map(([key,names]) => {
    const b = names.map(n=>sk.findBone(n)).find(Boolean);
    if (!b) throw Error(`${source.slug}: missing ${key}`);
    return [key,b];
  }));
  function pose(anim,t) {
    sk.setToSetupPose();
    anim.apply(sk,0,t,false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);
    sk.updateWorldTransform();
    return Object.fromEntries(Object.entries(bones).map(([key,b])=>[key,[b.worldX,b.worldY,b.getWorldRotationX()]]));
  }
  const anim = data.findAnimation('Spawn');
  if (!anim) throw Error(`${source.slug}: Spawn missing`);
  const idle = pose(data.findAnimation('Idle'),0);
  const height = Math.max(100,idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]));
  const count = Math.round(anim.duration*30), samples = [];
  let previous = {};
  for (let i=0;i<=count;i++) {
    const p = pose(anim,anim.duration*i/count), rotations = {};
    for (const key of Object.keys(bones)) {
      const parent = parents[key];
      const delta = (p[key][2]-idle[key][2])-(parent?p[parent][2]-idle[parent][2]:0);
      let angle = wrap(delta)*Math.PI/180;
      if (previous[key] !== undefined) angle=previous[key]+wrap((angle-previous[key])*180/Math.PI)*Math.PI/180;
      rotations[key] = round(angle);
    }
    previous = rotations;
    samples.push({rotations,root:[round((p.Hips[0]-idle.Hips[0])/height),round((p.Hips[1]-idle.Hips[1])/height)]});
  }
  records.push({id:source.id,slug:source.slug,clip:anim.name,duration:count/30,source:source.spine_sources.skeleton,
    sha256:crypto.createHash('sha256').update(bytes).digest('hex'),
    bones:Object.fromEntries(Object.entries(bones).map(([k,b])=>[k,b.data.name])),samples});
  console.log(source.slug,anim.name,count+'/30 seconds');
}
fs.mkdirSync('assets/characters/spawn',{recursive:true});
fs.writeFileSync('assets/characters/spawn/motions.json',JSON.stringify(records));
console.log('SPAWN_SAMPLES_OK', records.length);
