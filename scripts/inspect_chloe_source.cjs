const fs=require('fs'),vm=require('vm');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js','utf8'));
spine.MeshAttachment.prototype.updateRegion=spine.RegionAttachment.prototype.updateRegion=function(){};
const loader=Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n=>[`new${n}Attachment`,(s,name,path)=>new spine[`${n}Attachment`](name,path)]));
const data=new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(fs.readFileSync('assets/references/chloe-motion-reference.skel')));
console.log(JSON.stringify({animations:data.animations.map(a=>[a.name,a.duration]),bones:data.bones.map(b=>[b.name,b.parent?.name]),slots:data.slots.filter(s=>/Eye|Face|Sebastian/i.test(s.name)).map(s=>[s.name,s.attachmentName])},null,2));
