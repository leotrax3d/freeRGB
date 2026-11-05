import fs from 'node:fs';
const p='docs/OPENRGB_PORTING.md';
const lines=fs.readFileSync(p,'utf8').split('\n');
for(let i=0;i<lines.length;i++){
  if(lines[i].startsWith('| `DRGBController`')) {
    lines[i]='| `DRGBController` | partial | `ee37b8b` | 17 | V1 (6), V2 (5), V3 (6) Direct paths ported: V1 0x46/0xBB `0x47`/`0xA3`; V2 multi-report per 20 LEDs 0x00/0xBB; V3 `0x60`/0xBB `0x64`/`0xC8`. V4 (16) and 500 ms keepalive unported. |';
  }
}
fs.writeFileSync(p, lines.join('\n'));
console.log('done');
