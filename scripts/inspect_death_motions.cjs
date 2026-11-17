// Research only: inspect original battle skeletons already archived with their source URLs.
const fs = require('fs'), vm = require('vm');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js', 'utf8'));
spine.MeshAttachment.prototype.updateRegion = function () {};
spine.RegionAttachment.prototype.updateRegion = function () {};
const loader = Object.fromEntries(['Region','Mesh','BoundingBox','Path','Point','Clipping'].map(n =>
  [`new${n}Attachment`, (s,name,path) => new spine[`${n}Attachment`](name,path)]));
const sources = JSON.parse(fs.readFileSync('assets/references/manifest.json', 'utf8'));
const report = sources.map(source => {
  const data = new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(fs.readFileSync(`assets/references/${source.slug}-motion-reference.skel`)));
  const animation = data.findAnimation('Die');
  const result = {slug:source.slug, source:source.spine_sources.skeleton, duration:animation?.duration,
    animations:data.animations.map(a=>a.name), bones:data.bones.map(b=>({name:b.name,parent:b.parent?.name,x:b.x,y:b.y,rotation:b.rotation})),
    timelines:animation?.timelines.map(t=>({type:t.constructor.name,bone:data.bones[t.boneIndex]?.name,frames:Array.from(t.frames||[])}))};
  console.log(source.slug, animation?.duration, result.animations.join(', '));
  return result;
});
fs.writeFileSync('artifacts/death-replay/research/inspection.json',JSON.stringify(report,null,2));
