// Research only: evaluate every original standing eye attachment and its timing.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
const root = 'artifacts/cheek-eyes', input = 'artifacts/cheek-interaction/research';
fs.mkdirSync(root, {recursive: true});
vm.runInThisContext(fs.readFileSync(`${input}/spine-core.js`, 'utf8'));
const sources = JSON.parse(fs.readFileSync(`${input}/sources.json`));
async function download(url, file) {
  if (fs.existsSync(file)) return;
  const r = await fetch(url); if (!r.ok) throw new Error(`${r.status}: ${url}`);
  fs.writeFileSync(file, Buffer.from(await r.arrayBuffer()));
}
(async () => {
for (const source of sources) {
  const dir = `${root}/reference/${source.slug}`; fs.mkdirSync(dir, {recursive:true});
  await download(source.sources.atlas, `${dir}/skeleton.atlas`);
  const atlas = new spine.TextureAtlas(fs.readFileSync(`${dir}/skeleton.atlas`, 'utf8'));
  for (const p of atlas.pages) {
    await download(new URL(p.name, source.sources.atlas).href, `${dir}/${p.name}`);
    p.setTexture({getImage:()=>({width:p.width,height:p.height}),setFilters(){},setWraps(){}});
  }
  const bytes=fs.readFileSync(`${input}/${source.slug}.skel`);
  const data=new spine.SkeletonBinary(new spine.AtlasAttachmentLoader(atlas)).readSkeletonData(new Uint8Array(bytes));
  const sk=new spine.Skeleton(data);sk.setSkin(data.findSkin('Normal')||data.findSkin('S1')||data.skins[0]);
  function pose(anim,t) {sk.setToSetupPose();anim.apply(sk,0,t,false,[],1,spine.MixBlend.replace,spine.MixDirection.mixIn);sk.updateWorldTransform();}
  function meshes() {return sk.drawOrder.flatMap(slot=>{
    const a=slot.attachment;if(!a||slot.color.a<.01||!slot.bone.active)return [];
    let vertices,uvs,triangles;
    if(a instanceof spine.RegionAttachment){vertices=new Array(8);a.computeWorldVertices(slot,vertices,0,2);uvs=Array.from(a.uvs);triangles=[0,1,2,2,3,0];}
    else if(a instanceof spine.MeshAttachment){vertices=new Array(a.worldVerticesLength);a.computeWorldVertices(slot,0,vertices.length,vertices,0,2);uvs=Array.from(a.uvs);triangles=Array.from(a.triangles);}else return [];
    return [{name:slot.data.name,attachment:a.name,vertices,uvs,triangles,alpha:slot.color.a*a.color.a,page:a.region.page.name}];
  });}
  const frames=[],clips={};
  for(const name of ['Idle_1','Touch_Idle','Touch_End']) {
    const anim=data.findAnimation(name);if(!anim)throw new Error(`${source.slug}: ${name}`);
    const eyes=[];
    for(let frame=0;frame<=Math.ceil(anim.duration*30);frame++){
      const t=Math.min(frame/30,anim.duration);pose(anim,t);
      eyes.push({time:t,slots:sk.slots.filter(s=>/eye/i.test(s.data.name)&&!/(Seba|Drone|teddy|T_)/i.test(s.data.name)).map(s=>({slot:s.data.name,attachment:s.attachment?.name??null,scale:[s.bone.scaleX,s.bone.scaleY],alpha:s.color.a}))});
    }
    clips[name]={duration:anim.duration,eyes};
    for(const phase of (name==='Idle_1'?[0]:[.03,.25,.5,.75,.95])){pose(anim,phase*anim.duration);frames.push({clip:name,phase,meshes:meshes()});}
  }
  fs.writeFileSync(`${dir}/frames.json`,JSON.stringify(frames));
  fs.writeFileSync(`${dir}/eyes.json`,JSON.stringify({slug:source.slug,viewer:source.viewer,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),clips},null,2));
  console.log(source.slug, Object.fromEntries(Object.entries(clips).map(([name,c])=>[name,[...new Set(c.eyes.flatMap(f=>f.slots.map(s=>s.attachment).filter(Boolean)))]])));
}
})().catch(e=>{console.error(e);process.exit(1)});
