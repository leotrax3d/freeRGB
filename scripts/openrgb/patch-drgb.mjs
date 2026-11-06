import fs from 'node:fs';
const p='docs/OPENRGB_PORTING.md';
const lines=fs.readFileSync(p,'utf8').split('\n');
for(let i=0;i<lines.length;i++){
  if(lines[i].startsWith('| `DRGBController`')) {
    lines[i]='| `DRGBController` | partial | `6a5655e` | 6 | V1 path ported (6 detectors @ VID 0x2023): append-only Direct-mode frame using 65-byte reports with 0x46/0xBB header (per-zone 2-byte LED counts) and 0x47/0xA3 marker packets (`ceil(count/21)`). V2/V3/V4 and the 500 ms keepalive are still unported. |';
  }
}
fs.writeFileSync(p, lines.join('\n'));
console.log('done');
