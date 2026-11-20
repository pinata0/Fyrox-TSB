const fs=require('fs'),vm=require('vm');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js','utf8'));
spine.MeshAttachment.prototype.updateRegion=spine.RegionAttachment.prototype.updateRegion=function(){};
const loader=Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n=>[`new${n}Attachment`,(s,name,path)=>new spine[`${n}Attachment`](name,path)]));
const data=new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(fs.readFileSync('assets/references/renua-motion-reference.skel')));
console.log('BONES',data.bones.map(b=>b.name).join(', '));
console.log('SLOTS',data.slots.map(b=>b.name).join(', '));
for(const name of ['Attack1_1','Attack2_1','Skill1_1','Ultimate1_1']){
const a=data.findAnimation(name);console.log(name,a.duration,JSON.stringify(a.timelines.filter(t=>t.events).flatMap(t=>t.events.map(e=>({time:e.time,name:e.data.name,string:e.stringValue,int:e.intValue})))));
console.log('attachments',JSON.stringify(a.timelines.filter(t=>t.attachmentNames).map(t=>({slot:data.slots[t.slotIndex].name,times:Array.from(t.frames),names:t.attachmentNames}))));
}
