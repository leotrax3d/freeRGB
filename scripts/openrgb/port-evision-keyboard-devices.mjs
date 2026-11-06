// SPDX-License-Identifier: MIT OR Apache-2.0
// EVision keyboard generator: 15 detectors (13 V1 + 2 V2) on interface 1 with
// usage page 0xFF1C; emits devices_table.rs plus the V1 6x23 matrix map.
import fs from "node:fs";
import path from "node:path";
const [, , root = "openrgb", commit = "unknown"] = process.argv;
const family = path.join(root, "Controllers", "EVisionKeyboardController");
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
  /REGISTER_HID_DETECTOR_IP\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Z0-9_]+)\s*,\s*([A-Z0-9_]+)\s*,\s*1\s*,\s*([A-Z0-9_]+)\s*\)/g,
)) {
  detectors.push({
    name: m[1],
    handler: m[2],
    vid: num(m[3]),
    pid: num(m[4]),
    usagePage: num(m[5]),
    interface: 1,
    v2: m[2].includes("V2") || m[2].includes("Endorfy"),
  });
}
if (detectors.length !== 15) throw new Error(`expected 15 EVision detectors, found ${detectors.length}`);

// V1 6x23 matrix map (NA = unused cell) from RGBController_EVisionKeyboard.cpp
const rgbFile = path.join(family, 'EVisionV2KeyboardController', 'RGBController_EVisionKeyboard.cpp');
const rgb = fs.readFileSync(rgbFile, 'utf8');
const matrixStart = rgb.indexOf('matrix_map[') ;
const matrixEnd = rgb.indexOf('};', matrixStart);
const mx = rgb.slice(matrixStart, matrixEnd);
const dims = rgb.match(/matrix_map\[(\d+)\]\[(\d+)\]/);
const matrix = [...mx.matchAll(/\{([^}]*)\}/g)].map((r) =>
  [...r[1].matchAll(/0x[0-9A-Fa-f]+|\d+|NA/g)].map((t) => {
    const s = t[0];
    if (s === 'NA') return null;
    return s.toLowerCase().startsWith('0x') ? Number.parseInt(s.slice(2), 16) : Number.parseInt(s, 10);
  }),
);
if (matrix.length !== num(dims[1])) throw new Error(`matrix needs ${dims[1]} rows`);
const flat = matrix.flat();

const v1Rows = detectors.filter((d) => !d.v2);
const cells = flat.map((c) => (c == null ? 'None' : `Some(${c}u16)`));
const out = [
  '// SPDX-License-Identifier: GPL-2.0-or-later',
  `// Generated from OpenRGB ${commit}.`,
  'use super::table::Detector;',
  '',
  'pub static DETECTORS: &[Detector] = &[',
  ...detectors.map((d) =>
    `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: 0x${d.vid.toString(16).padStart(4, '0')}, pid: 0x${d.pid.toString(16).padStart(4, '0')}, v2: ${d.v2}, interface: ${d.interface}, usage_page: 0x${d.usagePage.toString(16)} },`,
  ),
  '];',
  '',
  '/// upstream: RGBController_EVisionKeyboard.cpp matrix_map (6 x 23, NA = None).',
  'pub static V1_MATRIX: &[Option<u16>] = &[',
  '    ' + cells.join(', '),
  '];',
  'pub static V1_MATRIX_HEIGHT: u16 = ' + num(dims[1]) + ';',
  'pub static V1_MATRIX_WIDTH: u16 = ' + num(dims[2]) + ';',
  '',
];
fs.writeFileSync("modules/evision-keyboard/src/devices_table.rs", out.join("\n"));
console.log(`generated ${detectors.length} EVision detectors and the V1 matrix`);