// Archive evaluated 2D bones, attachment expressions and rendered reference triangles.
const fs=require('fs'),vm=require('vm'),crypto=require('crypto');
const out='artifacts/chloe-skills/reference';fs.mkdirSync(out,{recursive:true});
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js','utf8'));
const atlas=new spine.TextureAtlas(fs.readFileSync(`${out}/skeleton.atlas`,'utf8'));
for(const page of atlas.pages)page.setTexture({getImage:()=>({width:page.width,height:page.height}),setFilters(){},setWraps(){}});
const bytes=fs.readFileSync('assets/references/chloe-motion-reference.skel');
const data=new spine.SkeletonBinary(new spine.AtlasAttachmentLoader(atlas)).readSkeletonData(new Uint8Array(bytes));
const sk=new spine.Skeleton(data);sk.setSkin(data.findSkin('Normal')||data.skins[0]);
const mapping=JSON.parse(fs.readFileSync('artifacts/basic-attacks/research/motion-report.json')).find(r=>r.slug==='chloe').bones;
Object.assign(mapping,{'Sebastian.Body':'T_body_2','Sebastian.Head':'T_head','Sebastian.UpperArm.L':'T_arm_1_L','Sebastian.Forearm.L':'T_arm_2_L','Sebastian.Hand.L':'T_arm_3_L','Sebastian.UpperArm.R':'T_arm_1_R','Sebastian.Forearm.R':'T_arm_2_R','Sebastian.Hand.R':'T_arm_3_R','Sebastian.Thigh.L':'teddy_leg_0_L','Sebastian.Shin.L':'teddy_leg_1_L','Sebastian.Foot.L':'teddy_leg_2_L','Sebastian.Thigh.R':'teddy_leg_0_R','Sebastian.Shin.R':'teddy_leg_1_R','Sebastian.Foot.R':'teddy_leg_2_R'});
const bones=Object.fromEntries(Object.entries(mapping).map(([k,n])=>[k,sk.findBone(n)]));
const round=x=>+x.toFixed(6);
function pose(anim,t){sk.setToSetupPose();anim.apply(sk,0,Math.min(anim.duration,t+1e-6),false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);sk.updateWorldTransform();return Object.fromEntries(Object.entries(bones).map(([k,b])=>[k,[round(b.worldX),round(b.worldY),round(b.getWorldRotationX())]]));}
function meshes(){return sk.drawOrder.flatMap(slot=>{const a=slot.attachment;if(!a||slot.color.a<.01||!slot.bone.active)return [];let vertices,uvs,triangles;if(a instanceof spine.RegionAttachment){vertices=new Array(8);a.computeWorldVertices(slot,vertices,0,2);uvs=Array.from(a.uvs);triangles=[0,1,2,2,3,0];}else if(a instanceof spine.MeshAttachment){vertices=new Array(a.worldVerticesLength);a.computeWorldVertices(slot,0,vertices.length,vertices,0,2);uvs=Array.from(a.uvs);triangles=Array.from(a.triangles);}else return [];return [{name:slot.data.name,attachment:a.name,vertices,uvs,triangles,alpha:slot.color.a*a.color.a}];});}
const idle=pose(data.findAnimation('Idle'),0),height=idle.Head[1]-Math.min(idle['Foot.L'][1],idle['Foot.R'][1]);
const clips=[],frames=[];
for(const name of ['Idle','Move','Skill1_1','Idle_SebastianForm','Move_SebastianForm','Skill1_1_SebastianForm','Skill2_1']){
 const anim=data.findAnimation(name),count=Math.round(anim.duration*30),samples=[];
 for(let i=0;i<=count;i++){const p=pose(anim,i/30);samples.push({bones:p,eyes:sk.slots.filter(s=>/eye|face/i.test(s.data.name)).map(s=>[s.data.name,s.attachment?.name??null])});}
 const events=anim.timelines.filter(t=>t.events).flatMap(t=>t.events.map(e=>[e.data.name,round(e.time)]));
 clips.push({clip:name,duration:count/30,events,samples});
 for(const phase of [0,.12,.26,.4,.54,.68,.82,.96]){pose(anim,phase*anim.duration);frames.push({clip:name,phase,meshes:meshes()});}
}
fs.writeFileSync('assets/characters/skill/chloe.json',JSON.stringify({sha256:crypto.createHash('sha256').update(bytes).digest('hex'),mapping,height,idle,clips}));
fs.writeFileSync(`${out}/frames.json`,JSON.stringify(frames));
fs.writeFileSync(`${out}/source.json`,JSON.stringify({mapping,height,clips:clips.map(({samples,...c})=>c)},null,2));
console.log(clips.map(c=>({clip:c.clip,frames:c.samples.length,events:c.events})));
