const source=require('fs').readFileSync('scripts/extract_ui_skills.cjs','utf8').split('const clips=[];')[0];
new Function('require',source+`
for(const n of ['Weapon_Leaf_0','Weapon_Leaf_1','Weapon_Leaf_2','Weapon_1','Weapon_2','Weapon_3']){
 const b=sk.findBone(n),a=[];
 for(let i=0;i<128;i++){pose(data.findAnimation('Ultimate1_1'),i/30);a.push([b.x,b.y,b.scaleX,b.scaleY,b.getWorldScaleX()]);}
 console.log(n,b.parent.data.name,a[0],a[0].map((_,j)=>[Math.min(...a.map(p=>p[j])),Math.max(...a.map(p=>p[j]))]));
}
console.log('SCALE_TIMELINES',data.findAnimation('Ultimate1_1').timelines.filter(t=>t.constructor.name.includes('Scale')).map(t=>[data.bones[t.boneIndex]?.name,Array.from(t.frames).slice(0,24)]));
`)(require);
