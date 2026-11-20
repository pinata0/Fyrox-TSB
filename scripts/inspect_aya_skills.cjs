const fs=require('fs'),vm=require('vm');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js','utf8'));
const atlas=new spine.TextureAtlas(fs.readFileSync('artifacts/aya-transform/reference/skeleton.atlas','utf8'));
for(const p of atlas.pages)p.setTexture({getImage:()=>({width:p.width,height:p.height}),setFilters(){},setWraps(){}});
const data=new spine.SkeletonBinary(new spine.AtlasAttachmentLoader(atlas)).readSkeletonData(new Uint8Array(fs.readFileSync('assets/references/aya-motion-reference.skel')));
console.log(JSON.stringify({bones:data.bones.map(b=>({name:b.name,parent:b.parent?.name})),slots:data.slots.map(s=>({name:s.name,bone:s.boneData.name})),clips:data.animations.filter(a=>/Skill1_1|Ultimate1_1/.test(a.name)).map(a=>({name:a.name,duration:a.duration,events:a.timelines.filter(t=>t.events).flatMap(t=>t.events.map(e=>({time:e.time,name:e.data.name,int:e.intValue,string:e.stringValue})))}))},null,2));
