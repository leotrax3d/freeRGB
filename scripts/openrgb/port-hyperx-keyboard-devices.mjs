// SPDX-License-Identifier: GPL-2.0-or-later
// Generator for the HyperX keyboard module device table and layouts.
// Usage: node scripts/openrgb/port-hyperx-keyboard-devices.mjs <openrgb-root> <commit>
// Reads Controllers/HyperXKeyboardController/*Detect.cpp and the layout tables
// of RGBController_HyperXAlloy{FPS,Elite}.cpp; regenerates
// modules/hyperx-keyboard/src/devices_table.rs and modules/hyperx-keyboard/src/layouts.rs.
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const root = process.argv[2] ?? process.env.OPENRGB_ROOT ?? '../openrgb';
const commit = process.argv[3] ?? 'unknown';
const family = path.join(root, 'Controllers', 'HyperXKeyboardController');
const detect = fs
  .readdirSync(family, { recursive: true })
  .filter((f) => /Detect\.cpp$/.test(f))
  .map((f) => fs.readFileSync(path.join(family, f), 'utf8'))
  .join('\n');

const headerText = fs
  .readdirSync(family, { recursive: true })
  .filter((f) => /\.h$/.test(f))
  .map((f) => fs.readFileSync(path.join(family, f), 'utf8'))
  .join('\n');
const constants = Object.fromEntries(
  [...detect.matchAll(/#define\s+(HYPERX[A-Z0-9_]*|HP_KEYBOARD_VID)\s+0x([0-9A-Fa-f]+)/g)]
    .concat(detect.matchAll(/#define\s+(HYPERX[A-Z0-9_]*|HP_KEYBOARD_VID)\s+0x([0-9A-Fa-f]+)/g))
    .map((m) => [m[1], Number.parseInt(m[2], 16)]),
);
const headerConstants = Object.fromEntries(
  [...headerText.matchAll(/#define\s+(HYPERX[A-Z0-9_]*|HP_KEYBOARD_VID)\s+0x([0-9A-Fa-f]+)/g)].map(
    (m) => [m[1], Number.parseInt(m[2], 16)],
  ),
);
const n = (s) => {
  const t = s.trim();
  if (t in constants) return constants[t];
  if (t in headerConstants) return headerConstants[t];
  if (/^0x[0-9A-Fa-f]+$/.test(t)) return Number.parseInt(t, 16);
  if (/^\d+$/.test(t)) return Number.parseInt(t, 10);
  throw new Error(`unresolved constant ${t}`);
};

// kind per detector handler, as used by the Rust module.
const kindOf = (handler) => {
  switch (handler) {
    case 'DetectHyperXAlloyElite':
      return 'elite';
    case 'DetectHyperXAlloyFPS':
      return 'fps';
    case 'DetectHyperXAlloyElite2':
      return 'elite2';
    case 'DetectHyperXAlloyOrigins':
    case 'DetectHyperXAlloyOrigins60and65':
      return 'origins';
    case 'DetectHyperXAlloyOriginsCore':
      return 'origins-core';
    case 'DetectHyperXEve1800':
      return 'eve';
    case 'DetectHyperXOrigins2_65':
      return 'origins2';
    default:
      throw new Error(`unknown handler ${handler}`);
  }
};

const PAGE_ANY = 0xffff; // sentinel in the Rust table for HID_USAGE_PAGE_ANY
const rows = [];
for (const m of detect.matchAll(
  /REGISTER_HID_DETECTOR(?:_I|_IP)?\s*\(\s*"([^"]+)"\s*,\s*([A-Za-z0-9_]+)\s*,\s*(HYPERX_KEYBOARD_VID|HP_KEYBOARD_VID)\s*,\s*([A-Z][A-Z0-9_]+)\s*(?:,\s*(\d+)\s*(?:,\s*0x([0-9A-Fa-f]+))?)?\s*\)/g,
)) {
  const vid = n(m[3]);
  const pid = n(m[4]);
  if (!Number.isFinite(vid) || !Number.isFinite(pid)) {
    throw new Error(`unresolved VID/PID ${m[3]}/${m[4]}`);
  }
  const iface = m[5] != null ? Number.parseInt(m[5], 10) : PAGE_ANY;
  const page = m[6] != null ? Number.parseInt(m[6], 16) : PAGE_ANY;
  rows.push({ name: m[1], handler: m[2], vid, pid, iface, page, kind: kindOf(m[2]) });
}
if (rows.length !== 20) throw new Error(`expected 20 detector rows, got ${rows.length}`);

// Layouts for the byte-exactly ported kinds (fps, elite).
const layouts = {};
for (const [dirName, kind, key] of [
  ['HyperXAlloyFPSController', 'fps', 'HyperXAlloyFPS'],
  ['HyperXAlloyEliteController', 'elite', 'HyperXAlloyElite'],
]) {
  const file = fs.readFileSync(path.join(family, dirName, `RGBController_${key}.cpp`), 'utf8');
  const zoneSizes = file.match(/zone_sizes\[\]\s*=\s*\{([^}]*)\}/)[1].split(',');
  const zoneTypes = file.match(/zone_types\[\]\s*=\s*\{([^}]*)\}/)[1].split(',');
  const zoneNames = file.match(/zone_names\[\]\s*=\s*\{([\s\S]*?)\n\};/)[1];
  const names = [...zoneNames.matchAll(/(ZONE_[A-Z_]+)/g)].map((m) => {
    const t = m[1];
    if (t === 'ZONE_EN_KEYBOARD') return 'Keyboard';
    if (t.startsWith('ZONE_EN_')) return t.replace(/^ZONE_EN_/, '').replace(/_/g, ' ').toLowerCase();
    return null;
  }).filter(Boolean);
  // matrix maps: first 6 x N block in the file
  const mx = file.match(/matrix_map\[\d+\]\[\d+\]\s*=\s*\{([\s\S]*?)\n\};/);
  const matrix = [...mx[1].matchAll(/\{([^}]*)\}/g)].map((r) =>
    r[1].split(',').map((t) => {
      const s = t.trim();
      return /^\d+$/.test(s) ? Number.parseInt(s, 10) : null;
    }),
  );
  const m = file.match(/matrix_map\[(\d+)\]\[(\d+)\]/);
  let ledNames = [];
  const ln = file.match(/led_names\[\]\s*=\s*\{([\s\S]*?)\n\};/);
  if (ln) {
    ledNames = [...ln[1].matchAll(/(KEY_[A-Z0-9_]+)/g)].map((x) => x[1].replace(/^KEY_EN_/, ''));
  }
  layouts[kind] = {
    zoneSizes: zoneSizes.map((s) => Number.parseInt(s.trim(), 10)),
    zoneTypes: zoneTypes.map((s) => s.trim().replace('ZONE_TYPE_', '')),
    zoneNames: names,
    matrixWidth: Number.parseInt(m[2], 10),
    matrixHeight: Number.parseInt(m[1], 10),
    matrix,
    ledNames,
  };
}

const out = path.join(process.cwd(), 'modules', 'hyperx-keyboard', 'src');
fs.mkdirSync(out, { recursive: true });
fs.writeFileSync(
  path.join(out, 'devices_table.rs'),
  [
    '// SPDX-License-Identifier: GPL-2.0-or-later',
    `// Generated by scripts/openrgb/port-hyperx-keyboard-devices.mjs from OpenRGB ${commit}; do not edit.`,
    'use crate::table::Detector;',
    '',
    '// interface 255 = any interface; usage_page 255 = any usage page.',
    'pub const DETECTORS: &[Detector] = &[',
    ...rows.map((d) =>
      `    Detector { name: ${JSON.stringify(d.name)}, handler: ${JSON.stringify(d.handler)}, vid: 0x${d.vid.toString(16).padStart(4, '0')}, pid: 0x${d.pid.toString(16).padStart(4, '0')}, interface: ${d.iface}, usage_page: ${d.page}, kind: ${JSON.stringify(d.kind)} },`,
    ),
    '];',
    '',
  ].join('\n'),
);

const L = layouts;
const layoutLines = [
  '// SPDX-License-Identifier: GPL-2.0-or-later',
  `// Generated by scripts/openrgb/port-hyperx-keyboard-devices.mjs from OpenRGB ${commit}; do not edit.`,
  '// Zone sizes, types, matrix maps and LED names from the upstream RGBControllers.',
  'pub struct Layout {',
  '    pub zone_names: &\'static [&\'static str],',
  '    pub zone_sizes: &\'static [usize],',
  '    pub zone_types: &\'static [&\'static str],',
  '    pub matrix_width: u16,',
  '    pub matrix_height: u16,',
  '    pub matrix: &\'static [Option<u16>],',
  '    pub led_names: &\'static [&\'static str],',
  '}',
  '',
  'pub fn layout(kind: &str) -> Option<Layout> {',
  '    match kind {',
];
for (const [kind, l] of Object.entries(L)) {
  layoutLines.push(`        "${kind}" => Some(Layout {`);
  layoutLines.push(`            zone_names: &[${l.zoneNames.map((x) => JSON.stringify(x)).join(', ')}],`);
  layoutLines.push(`            zone_sizes: &[${l.zoneSizes.join(', ')}],`);
  layoutLines.push(`            zone_types: &[${l.zoneTypes.map((x) => JSON.stringify(x)).join(', ')}],`);
  layoutLines.push(`            matrix_width: ${l.matrixWidth},`);
  layoutLines.push(`            matrix_height: ${l.matrixHeight},`);
  layoutLines.push(
    `            matrix: &[${l.matrix.flat().map((c) => (c == null ? 'None' : `Some(${c}u16)`)).join(', ')}],`,
  );
  layoutLines.push(`            led_names: &[${l.ledNames.map((x) => JSON.stringify(x)).join(', ')}],`);
  layoutLines.push('        }),');
}
layoutLines.push('        _ => None,', '    }', '}', '');
fs.writeFileSync(path.join(out, 'layouts.rs'), layoutLines.join('\n'));
execFileSync(
  process.platform === 'win32' ? 'rustfmt.exe' : 'rustfmt',
  [path.join(out, 'devices_table.rs'), path.join(out, 'layouts.rs')],
  { stdio: 'ignore' },
);
console.log(`generated ${rows.length} HyperX keyboard HID rows and layouts for ${Object.keys(L).join('/')}`);