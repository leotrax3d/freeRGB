import fs from 'node:fs';
const p = 'docs/OPENRGB_PORTING.md';
const lines = fs.readFileSync(p, 'utf8').split('\n');
const bt = '`';
for (let i = 0; i < lines.length; i++) {
  if (lines[i].includes('PatriotViperMouseController') && lines[i].includes('605c58e')) {
    lines[i] = lines[i].replace('605c58e', bt + '605c58e' + bt);
  }
}
fs.writeFileSync(p, lines.join('\n'));
console.log('done');
