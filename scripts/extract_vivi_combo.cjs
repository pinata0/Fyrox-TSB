// Evaluate both archived EPID Vivi attacks, including IK and independent floating blades.
const fs = require('fs'), vm = require('vm'), crypto = require('crypto');
vm.runInThisContext(fs.readFileSync('artifacts/cheek-interaction/research/spine-core.js', 'utf8'));
spine.MeshAttachment.prototype.updateRegion = spine.RegionAttachment.prototype.updateRegion = function () {};
const loader = Object.fromEntries(['Region', 'Mesh', 'BoundingBox', 'Path', 'Point', 'Clipping'].map(n =>
  [`new${n}Attachment`, (s, name, path) => new spine[`${n}Attachment`](name, path)]));
const bytes = fs.readFileSync('assets/references/vivi-motion-reference.skel');
const data = new spine.SkeletonBinary(loader).readSkeletonData(new Uint8Array(bytes));
const sk = new spine.Skeleton(data); sk.setSkin(data.findSkin('Normal') || data.skins[0]);
const mapping = JSON.parse(fs.readFileSync('artifacts/basic-attacks/research/motion-report.json'))[0].bones;
const bones = Object.fromEntries(Object.entries(mapping).map(([k, n]) => [k, sk.findBone(n)]));
const blades = ['Weapon_Dagger1', 'Weapon_Dagger3'].map(n => sk.findBone(n));
const parents = { Hips: null, Torso: 'Hips', Head: 'Torso' };
for (const s of ['L', 'R']) Object.assign(parents, {
  [`UpperArm.${s}`]: 'Torso', [`Forearm.${s}`]: `UpperArm.${s}`, [`Hand.${s}`]: `Forearm.${s}`,
  [`Thigh.${s}`]: 'Hips', [`Shin.${s}`]: `Thigh.${s}`, [`Foot.${s}`]: `Shin.${s}`,
});
const round = x => +x.toFixed(6), wrap = x => ((x + 180) % 360 + 360) % 360 - 180;
function pose(anim, time) {
  sk.setToSetupPose(); anim.apply(sk, 0, time, false, [], 1, spine.MixBlend.replace, spine.MixDirection.mixIn);
  sk.updateWorldTransform();
  return Object.fromEntries([...Object.entries(bones), ...blades.map((b, i) => [`Blade${i}`, b])]
    .map(([key, b]) => [key, [b.worldX, b.worldY, b.getWorldRotationX()]]));
}
const idle = pose(data.findAnimation('Idle'), 0);
const height = Math.max(100, idle.Head[1] - Math.min(idle['Foot.L'][1], idle['Foot.R'][1]));
const records = [], reports = [];
for (const name of ['Attack1_1', 'Attack2_1']) {
  const anim = data.findAnimation(name), count = Math.round(anim.duration * 30), duration = count / 30;
  const samples = [], points = []; let previous = {}, previousHands = [], previousBlades = [];
  for (let i = 0; i <= count; i++) {
    const p = pose(anim, duration * i / count), rotations = {};
    for (const key of Object.keys(bones)) {
      const parent = parents[key];
      let angle = wrap((p[key][2] - idle[key][2]) - (parent ? p[parent][2] - idle[parent][2] : 0)) * Math.PI / 180;
      if (previous[key] !== undefined) angle = previous[key] + wrap((angle - previous[key]) * 180 / Math.PI) * Math.PI / 180;
      rotations[key] = round(angle);
    }
    previous = rotations;
    function path(key, index, prev) {
      const point = p[key], rest = idle[key];
      let angle = wrap(point[2] - rest[2]) * Math.PI / 180;
      if (prev[index] !== undefined) angle = prev[index] + wrap((angle - prev[index]) * 180 / Math.PI) * Math.PI / 180;
      prev[index] = angle;
      return [round((point[0] - p.Torso[0] - rest[0] + idle.Torso[0]) / height),
        round((point[1] - p.Torso[1] - rest[1] + idle.Torso[1]) / height), round(angle)];
    }
    samples.push({ rotations, root: [round((p.Hips[0] - idle.Hips[0]) / height), round((p.Hips[1] - idle.Hips[1]) / height)],
      hands: ['L', 'R'].map((s, j) => path(`Hand.${s}`, j, previousHands)),
      blades: blades.map((_, j) => path(`Blade${j}`, j, previousBlades)) });
    points.push(p);
  }
  const events = anim.timelines.filter(t => t.events).flatMap(t => t.events.filter(e => e.data.name === 'Event').map(e => round(e.time)));
  records.push({ slug: 'vivi', clip: name, duration, events, samples });
  reports.push({ clip: name, duration, events, bones: mapping, blade_bones: blades.map(b => b.data.name), points });
  console.log(name, duration, 'events', events);
}
fs.mkdirSync('artifacts/vivi-swordplay/research', { recursive: true });
fs.writeFileSync('assets/characters/attack/vivi-combo.json', JSON.stringify(records));
fs.writeFileSync('artifacts/vivi-swordplay/research/motions.json', JSON.stringify({
  viewer: 'https://wiki.cheeky.tsuncloud.com/korea/spines/vivi',
  sha256: crypto.createHash('sha256').update(bytes).digest('hex'), idle, height, clips: reports,
}));
