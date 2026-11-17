// Offline research: evaluate the archived EPID battle tracks, including IK and curves.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js', 'utf8'));
spine.MeshAttachment.prototype.updateRegion = spine.RegionAttachment.prototype.updateRegion = function () {};
const loader = Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n =>
  [`new${n}Attachment`, (s,name,path) => new spine[`${n}Attachment`](name,path)]));
const mapping = {
  Hips:['Body_1','body_1'], Torso:['Body_2','body_2'], Head:['Head','head'],
  'UpperArm.L':['UpArm_R','arm_1_R'], 'Forearm.L':['Arm_R','arm_2_R'], 'Hand.L':['Hand_R','hand_R','hand_1_R'],
  'UpperArm.R':['UpArm_L','arm_1_L'], 'Forearm.R':['Arm_L','arm_2_L'], 'Hand.R':['Hand_L','hand_L'],
  'Thigh.L':['UpLeg_R','leg_1_R'], 'Shin.L':['Leg_R','leg_2_R'], 'Foot.L':['Foot_R','foot_R','Leg_R','leg_2_R'],
  'Thigh.R':['UpLeg_L','leg_1_L'], 'Shin.R':['Leg_L','leg_2_L'], 'Foot.R':['Foot_L','foot_L','Leg_L','leg_2_L'],
};
const parents={Hips:null,Torso:'Hips',Head:'Torso'};
for(const s of ['L','R']) Object.assign(parents,{
  [`UpperArm.${s}`]:'Torso',[`Forearm.${s}`]:`UpperArm.${s}`,[`Hand.${s}`]:`Forearm.${s}`,
  [`Thigh.${s}`]:'Hips',[`Shin.${s}`]:`Thigh.${s}`,[`Foot.${s}`]:`Shin.${s}`,
});
const round=x=>+x.toFixed(6), wrap=x=>((x+180)%360+360)%360-180;
const records=[], reports=[];
for(const source of JSON.parse(fs.readFileSync('assets/references/manifest.json','utf8'))){
  const bytes=fs.readFileSync(`assets/references/${source.slug}-motion-reference.skel`);
  const data=new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
  const sk=new spine.Skeleton(data); sk.setSkin(data.findSkin('Normal')||data.skins[0]);
  const bones=Object.fromEntries(Object.entries(mapping).map(([key,names])=>{
    // Battle source rigs label the weapon hand Hand_L (Weapon1_1_T/Gun_body2).
    // Butter's fork instead follows Hand_R; preserve that explicitly.
    const sourceNames=source.slug==='butter' ? names : names.map(n=>n.replace(/_([LR])$/,(_,s)=>'_'+(s==='L'?'R':'L')));
    if(key.startsWith('Hand.'))sourceNames.push('hand_1_'+sourceNames[0].slice(-1));
    const b=sourceNames.map(n=>sk.findBone(n)).find(Boolean);
    if(!b)throw Error(`${source.slug}: missing ${key}`);
    return [key,b];
  }));
  function pose(anim,t){
    sk.setToSetupPose(); anim.apply(sk,0,t,false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn); sk.updateWorldTransform();
    return Object.fromEntries(Object.entries(bones).map(([key,b])=>[key,[b.worldX,b.worldY,b.getWorldRotationX()]]));
  }
  const idle=pose(data.findAnimation('Idle'),0), anim=data.findAnimation('Attack1_1');
  if(!anim) throw Error(`${source.slug}: no normal attack`);
  const height=Math.max(100,idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]));
  const count=Math.round(anim.duration*30), duration=count/30;
  const samples=[],points=[]; let previous={}, previousHands=[];
  for(let i=0;i<=count;i++){
    const p=pose(anim,duration*i/count), rotations={};
    for(const key of Object.keys(bones)){
      const parent=parents[key];
      const delta=(p[key][2]-idle[key][2])-(parent?p[parent][2]-idle[parent][2]:0);
      let a=wrap(delta)*Math.PI/180;
      if(previous[key]!==undefined) a=previous[key]+wrap((a-previous[key])*180/Math.PI)*Math.PI/180;
      rotations[key]=round(a);
    }
    previous=rotations;
    // Source plane X is forward, Y is up; depth is inferred during 3D retargeting.
    const hands=['L','R'].map((side,index)=>{
      const key=`Hand.${side}`, hand=p[key], rest=idle[key];
      let angle=wrap(hand[2]-rest[2])*Math.PI/180;
      if(previousHands[index]!==undefined) angle=previousHands[index]+wrap((angle-previousHands[index])*180/Math.PI)*Math.PI/180;
      previousHands[index]=angle;
      return [round((hand[0]-p.Torso[0]-rest[0]+idle.Torso[0])/height),
        round((hand[1]-p.Torso[1]-rest[1]+idle.Torso[1])/height),
        round(angle)];
    });
    samples.push({rotations,root:[round((p.Hips[0]-idle.Hips[0])/height),round((p.Hips[1]-idle.Hips[1])/height)],hands});
    points.push(p);
  }
  const events=anim.timelines.filter(t=>t.events).flatMap(t=>t.events.filter(e=>e.data.name==='Event').map(e=>round(e.time)));
  records.push({slug:source.slug,clip:anim.name,duration,events,samples});
  reports.push({slug:source.slug,clip:anim.name,duration,events,source:source.spine_sources.skeleton,viewer:source.motion_viewer,
    sha256:crypto.createHash('sha256').update(bytes).digest('hex'),
    bones:Object.fromEntries(Object.entries(bones).map(([k,v])=>[k,v.data.name])),
    alternatives:data.animations.filter(a=>/Attack/.test(a.name)).map(a=>({name:a.name,duration:a.duration})),points});
  console.log(source.slug,anim.name,duration.toFixed(3),JSON.stringify(events));
}
fs.mkdirSync('assets/characters/attack',{recursive:true}); fs.mkdirSync('artifacts/basic-attacks/research',{recursive:true});
fs.writeFileSync('assets/characters/attack/motions.json',JSON.stringify(records));
fs.writeFileSync('artifacts/basic-attacks/research/motion-report.json',JSON.stringify(reports));
