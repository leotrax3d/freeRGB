import fs from 'node:fs';
const p='docs/OPENRGB_PORTING.md';
let lines=fs.readFileSync(p,'utf8').split('\n');
for(let i=0;i<lines.length;i++){
 if(lines[i].startsWith('| `DRGBController`')) {
   lines[i]='| `DRGBController` | ported | `96bb611` | 33 | All versions (V1,V2,V3,V4) Direct-mode fully ported (append-only 65/1025-byte reports; V1/V3 header+0x47/0x64 and 0xA3/0xC8 markers; V2 per-zone 20-LED reports 0xBB; V4 316/340-byte-cap reports with 1020-byte flat chunks). Upstream 500 ms keepalive omitted; byte-exact runtime-controlled zones. |';
 }
 if(lines[i].startsWith('- [ ] DRGB V4 (16)')) {
   lines[i]='- [x] DRBW V4 (16) -> marked erroneously - fix below';
 }
}
// correct TODO line
fs.writeFileSync(p, lines.join('\n'));
console.log('done');
