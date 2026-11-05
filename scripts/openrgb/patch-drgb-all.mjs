import fs from 'node:fs';
const p='docs/OPENRGB_PORTING.md';
const lines=fs.readFileSync(p,'utf8').split('\n');
for(let i=0;i<lines.length;i++){
  if(lines[i].startsWith('| `DRGBController`')) {
    lines[i]='| `DRGBController` | ported | TBD | 33 | All versions (V1, V2, V3, V4) Direct-mode ported (append-only 65/1025-byte reports; V1/V3 header+0x47/0x64 and 0xA3/0xC8; V2 zone reports 0xBB; V4 316/340 + 948 packed bytes at i*1020). Upstream 500 ms keepalive omitted; default per-channel LED allocation avoided by using runtime zone counts. |';
  }
}
fs.writeFileSync(p, lines.join('\n'));
console.log('done');
