import fs from 'node:fs';
const root = process.argv[2] ?? 'openrgb';
const src = fs.readFileSync(root+'/Controllers/CorsairPeripheralController/CorsairK55RGBPROXTController.cpp','utf8');
const m = src.match(/static const unsigned int keys\[\] =\s*\{([\s\S]*?)\};/);
if(!m) throw new Error('hex');
const nums = [...m[1].matchAll(/\b\d+\b/g)].map(x=>parseInt(x[0],10));
if(nums.length!==111) throw new Error(`bad count ${nums.length}`);
const out = [
  '// SPDX-License-Identifier: GPL-2.0-or-later',
  'pub const KEYS: [usize; 111] = [',
  nums.map(n=>'    '+n+',').join('\n'),
  '];',
].join('\n');
fs.mkdirSync('modules/corsair-k55-pro-xt/src', { recursive: true });
fs.writeFileSync('modules/corsair-k55-pro-xt/src/keys.rs', out);
console.log('generated 111 keys');
