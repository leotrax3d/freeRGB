import fs from 'node:fs';
import path from 'node:path';
const root = process.argv[2] ?? 'openrgb', commit = process.argv[3] ?? 'unknown';
const rgbPath = path.join(root, 'Controllers', 'DarkProject', 'DarkProjectKeyboardController.cpp');
const src = fs.readFileSync(rgbPath, 'utf8');
const m = src.match(/static uint8_t packet_map\[\d+\]\s*=\s*\{([\s\S]*?)\};/);
if (!m) throw new Error('packet_map not found');
let mapSrc = m[1].replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '');
const numbers = [...mapSrc.matchAll(/\b\d+\b/g)].map(x => parseInt(x[0], 10));
if (numbers.length !== 87) throw new Error(`expected 87 packet_map values, found ${numbers.length}`);
const out = [
  '// SPDX-License-Identifier: GPL-2.0-or-later',
  `// Generated from OpenRGB ${commit}.`,
  `pub const PACKET_MAP: [usize; 87] = [`,
  numbers.map(n => '    ' + n + ',').join('\n'),
  '];',
];
fs.mkdirSync('modules/dark-project/src', { recursive: true });
fs.writeFileSync('modules/dark-project/src/packet_map.rs', out.join('\n'));
console.log(`generated packet_map with ${numbers.length} entries`);
