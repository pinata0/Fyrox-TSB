// Evaluate archived Aya battle tracks, including attachment changes, IK and curves.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
const out = 'artifacts/aya-skills';
fs.mkdirSync(`${out}/reference`, {recursive:true});
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js','utf8'));
const atlas = new spine.TextureAtlas(fs.readFileSync('artifacts/aya-transform/reference/skeleton.atlas','utf8'));
for (const page of atlas.pages) page.setTexture({getImage:()=>({width:page.width,height:page.height}),setFilters(){},setWraps(){}});
const bytes = fs.readFileSync('assets/references/aya-motion-reference.skel');
const data = new spine.SkeletonBinary(new spine.AtlasAttachmentLoader(atlas)).readSkeletonData(new Uint8Array(bytes));
const sk = new spine.Skeleton(data); sk.setSkin(data.findSkin('Normal') || data.skins[0]);
const mapping = JSON.parse(fs.readFileSync('artifacts/basic-attacks/research/motion-report.json')).find(r=>r.slug==='aya').bones;
const bones = Object.fromEntries(Object.entries(mapping).map(([key,n])=>[key,sk.findBone(n)]));
const parents={Hips:null,Torso:'Hips',Head:'Torso'};
for(const s of ['L','R']) Object.assign(parents,{[`UpperArm.${s}`]:'Torso',[`Forearm.${s}`]:`UpperArm.${s}`,[`Hand.${s}`]:`Forearm.${s}`,[`Thigh.${s}`]:'Hips',[`Shin.${s}`]:`Thigh.${s}`,[`Foot.${s}`]:`Shin.${s}`});
const round=x=>+x.toFixed(6), wrap=x=>((x+180)%360+360)%360-180;
function pose(anim,t){sk.setToSetupPose();anim.apply(sk,0,Math.min(anim.duration,t+1e-6),false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);sk.updateWorldTransform();return Object.fromEntries(Object.entries(bones).map(([k,b])=>[k,[b.worldX,b.worldY,b.getWorldRotationX()]]));}
function meshes(){return sk.drawOrder.flatMap(slot=>{
 const a=slot.attachment;if(!a||slot.color.a<.01||!slot.bone.active)return [];
 let vertices,uvs,triangles;
 if(a instanceof spine.RegionAttachment){vertices=new Array(8);a.computeWorldVertices(slot,vertices,0,2);uvs=Array.from(a.uvs);triangles=[0,1,2,2,3,0];}
 else if(a instanceof spine.MeshAttachment){vertices=new Array(a.worldVerticesLength);a.computeWorldVertices(slot,0,vertices.length,vertices,0,2);uvs=Array.from(a.uvs);triangles=Array.from(a.triangles);}else return [];
 return [{name:slot.data.name,attachment:a.name,vertices,uvs,triangles,alpha:slot.color.a*a.color.a}];
});}
const idle=pose(data.findAnimation('Idle'),0), height=idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]);
const detailNames=['skirt_root',...Array.from({length:4},(_,j)=>Array.from({length:4},(_,k)=>`Skirt${j+1}_${k}`)).flat(),'Hair_06_0','Hair_06_1','Hair_08_0','Hair_08_1'];
const rest=detailNames.map(n=>sk.findBone(n).rotation);
const records=[],frames=[],debug=[];
for(const name of ['Skill1_1','Ultimate1_1']){
 const anim=data.findAnimation(name),count=Math.round(anim.duration*30),samples=[];let previous={};
 for(let i=0;i<=count;i++){
  const phase=i/count,p=pose(anim,anim.duration*phase),rotations={};
  for(const key of Object.keys(bones)){
   const parent=parents[key];let a=wrap((p[key][2]-idle[key][2])-(parent?p[parent][2]-idle[parent][2]:0))*Math.PI/180;
   if(previous[key]!==undefined)a=previous[key]+wrap((a-previous[key])*180/Math.PI)*Math.PI/180;
   rotations[key]=round(a);
  }
  previous=rotations;
  const hands=['L','R'].map(s=>{const k=`Hand.${s}`;return [round((p[k][0]-p.Torso[0]-idle[k][0]+idle.Torso[0])/height),round((p[k][1]-p.Torso[1]-idle[k][1]+idle.Torso[1])/height),rotations[k]+rotations[`Forearm.${s}`]+rotations[`UpperArm.${s}`]+rotations.Torso+rotations.Hips];});
  const sword=sk.findSlot('Sword'),stem=sk.findBone('Weapon_Stem');
  const fold=Math.max(0,Math.min(1,sk.findBone('Weapon_Petal_01_0').rotation/68.764328));
  const spread=Math.max(0,Math.min(1,(sk.findBone('Weapon_Petal_Root').scaleX-.509494)/(.890506)))*fold;
  const details=detailNames.map((n,j)=>round(wrap(sk.findBone(n).rotation-rest[j])*Math.PI/180));
  samples.push({details,rotations,root:[round((p.Hips[0]-idle.Hips[0])/height),round((p.Hips[1]-idle.Hips[1])/height)],hands,fold:round(fold),spread:round(spread),blade:sword.attachment?Number(sword.attachment.name.slice(-1)):0,blade_alpha:round(sword.color.a),weapon_angle:round(wrap(stem.getWorldRotationX()-p['Hand.L'][2])*Math.PI/180)});
 }
 const attachments=anim.timelines.filter(t=>t.attachmentNames&&/Sword|Weapon/.test(data.slots[t.slotIndex].name)).map(t=>({slot:data.slots[t.slotIndex].name,times:Array.from(t.frames).map(round),names:t.attachmentNames}));
 records.push({slug:'aya',clip:name,duration:count/30,attachments,samples});
 for(const phase of [0,.1,.2,.3,.4,.52,.65,.82,1]){
  pose(anim,anim.duration*phase);frames.push({clip:name,phase,meshes:meshes()});
  debug.push({clip:name,phase,bones:sk.bones.filter(b=>/Weapon_(Stem|Petal_(Root|FCT|BCT|01_|04_|08_))|Sword/.test(b.data.name)).map(b=>({name:b.data.name,rot:round(b.rotation),scale:[round(b.scaleX),round(b.scaleY)],xy:[round(b.x),round(b.y)]})),sword:{name:sk.findSlot('Sword').attachment?.name,alpha:sk.findSlot('Sword').color.a}});
 }
}
fs.writeFileSync(`${out}/reference/frames.json`,JSON.stringify(frames));
fs.writeFileSync(`${out}/reference/bones.json`,JSON.stringify(debug,null,2));
fs.writeFileSync('assets/characters/skill/aya.json',JSON.stringify({detailNames,clips:records}));
fs.writeFileSync(`${out}/reference/source.json`,JSON.stringify({sha256:crypto.createHash('sha256').update(bytes).digest('hex'),mapping,idle,height,detailNames,clips:records.map(({samples,...r})=>r)},null,2));
console.log(records.map(r=>({clip:r.clip,duration:r.duration,attachments:r.attachments})));

