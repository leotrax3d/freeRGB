import fs from 'node:fs';
const arr=[41,43,57,106,105,30,100,108,31,20,4,29,107,32,26,22,27,33,8,7,6,0,34,21,9,25,35,23,10,5,44,36,28,11,17,37,24,13,16,1,38,12,14,54,39,18,15,55,111,45,19,51,56,122,46,47,52,101,48,50,42,40,110,109];
if(arr.length!==64) throw new Error('bad length '+arr.length);
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
'pub const POSITIONS: [usize; 64] = [',
arr.map(n=>'    '+n+',').join('\n'),
'];',
];
fs.mkdirSync('modules/corsair-k65-mini/src',{recursive:true});
fs.writeFileSync('modules/corsair-k65-mini/src/positions.rs',out.join('\n'));
console.log('generated 64 positions');