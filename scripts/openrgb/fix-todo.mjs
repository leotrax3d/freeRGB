import fs from 'node:fs';
const p='docs/OPENRGB_PORTING.md';
const t=fs.readFileSync(p,'utf8');
const idx=t.indexOf('## TODO / Fortschritt');
if(idx>=0){
 const head=t.slice(0, idx).trimEnd();
 const todo=`\n\n## TODO / Fortschritt (freeRGB Portierung)

- [x] Teil A – Aufräumen/Audit (Mountain, ThingM)
- [x] HyperX Dart
- [x] Dream Cheeky
- [x] Clevo Lightbar
- [x] Tecknet M008
- [x] MSI 3‑Zone
- [x] Dark Project KD3B V2
- [x] Corsair K55 RGB PRO XT
- [x] Corsair K65 Mini
- [x] Corsair Peripheral Generic – Mouse/Mat/ST100 (+16)
- [ ] Corsair Peripheral Generic – Keyboards STREAM 0x7F (blockiert)
- [x] Patriot Viper V550
- [x] DRGB V1–V4 (33, Direct complete, keepalive omitted)
- [x] GameSir
- [x] Lexip Np93 ALPHA
- [x] MadCatz Cyborg
- [x] Gigabyte Aorus Mouse
- [x] N5312A
- [x] Ionico
- [x] NZXT Mouse
- [x] AOC Mousemat
- [ ] CoolerMasterController (38)
- [ ] RoccatController (28)
- [ ] AsusAuraUSB (ohne 0b05:19af)
- [ ] weitere kleine Familien (Cougar, LegoDimensions, RealtekARGB, Skydimo, XPG Summoner …)

Last update: DRGB all versions ported.
`;
 fs.writeFileSync(p, head+todo);
}
console.log('done');
