// SPDX-License-Identifier: MIT OR Apache-2.0
// Royuan keyboard generator: 5 detector rows with interfaces per the detect file.
import fs from "node:fs";
import path from "node:path";
const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "RoyuanKeyboardController");
const files = [];
function walk(dir) { for (const e of fs.readdirSync(dir, { withFileTypes: true })) { const p = path.join(dir, e.name); if (e.isDirectory()) walk(p); else if (/\.(cpp|h)$/.test(e.name)) files.push(p); } }
walk(family);
const source = files.map((f) => fs.readFileSync(f, "utf8")).join("\n");
const values = new Map();
for (const m of source.matchAll(/^\s*#define\s+([A-Z0-9_]+)\s+(0x[0-9A-Fa-f]+|\d+)/gm)) {
  values.set(m[1], m[2].startsWith('0x') ? Number.parseInt(m[2], 16) : Number.parseInt(m[2], 10));
}
const num = (t) => {
  if (values.has(t)) return values.get(t);
  if (/^0x[0-9A-Fa-f]+$/.test(t)) return Number.parseInt(t, 16);
  if (/^\d+$/.test(t)) return Number.parseInt(t, 10);
  throw new Error(`unresolved constant ${t}`);
};

const detectors = [];
for (const m of source.matchAll(
  /REGISTER_HID_DETECTOR_I\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*ROYUAN_KEYBOARD_VID\s*,\s*([A-Z0-9_]+)\s*,\s*(\d+)\s*\)/g,
)) {
  const protocol = m[2].includes('GK68HEPro')
    ? 'skyloong'
    : m[2].includes('Akko')
      ? 'akko'
      : 'legacy';
  detectors.push({
    name: m[1],
    handler: m[2],
    vid: num('ROYUAN_KEYBOARD_VID'),
    pid: num(m[3]),
    interface: Number.parseInt(m[4], 10),
    protocol,
  });
}
if (detectors.length !== 5) throw new Error(`expected 5 Royuan detectors, found ${detectors.length}`);

const out = [
  '// SPDX-License-Identifier: GPL-2.0-or-later',
  `// Generated from OpenRGB ${commit}.`,
  'use super::table::Detector;',
  '',
  'pub static DETECTORS: &[Detector] = &[',
  ...detectors.map(
    (d) =>
      `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: 0x${d.vid.toString(16).padStart(4, '0')}, pid: 0x${d.pid.toString(16).padStart(4, '0')}, interface: ${d.interface}, protocol: ${JSON.stringify(d.protocol)} },`,
  ),
  '];',
  '',
];
fs.writeFileSync("modules/royuan-keyboard/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} Royuan keyboard detectors`);