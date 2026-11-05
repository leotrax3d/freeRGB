import fs from 'node:fs';
const p='docs/OPENRGB_PORTING.md';
const lines=fs.readFileSync(p,'utf8').split('\n');
for(let i=0;i<lines.length;i++){
  if(lines[i].startsWith('| `DRGBController`')) {
    lines[i]='| `DRGBController` | partial | `49a6e9e` | 11 | V1 (6) and V2 (5) Direct paths ported: V1 header + 63-byte `0x47`/`0xA3` (i+92) packets; V2 zone-major reports `[Curr,Num,zone,0xBB]+60 bytes (20 LEDs/60-byte)`. V3/V4 (16) and the 500 ms keepalive remain unported. |';
  }
}
fs.writeFileSync(p, lines.join('\n'));
console.log('done');
