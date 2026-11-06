import fs from 'node:fs';
const p='docs/OPENRGB_PORTING.md';
let lines=fs.readFileSync(p,'utf8').split(/\r?\n/);
for(let i=0;i<lines.length;i++){
  if(lines[i].startsWith('| `CorsairPeripheralController` |')) {
    lines[i]='| `CorsairPeripheralController` | partial+blocked | `4ab24c3` | 18 | Generic mouse path ported; keyboard STREAM 0x7F path blocked (shared KeyboardLayoutManager + 42K key maps). See modules/corsair-peripheral. |';
  }
  if(lines[i].startsWith('- [ ] Corsair Peripheral Generic')) {
    lines[i]='- [ ] Corsair Peripheral Generic ~ blocked (shared KeyboardLayoutManager/42K key maps)';
  }
}
fs.writeFileSync(p, lines.join('\n'));
console.log('done');