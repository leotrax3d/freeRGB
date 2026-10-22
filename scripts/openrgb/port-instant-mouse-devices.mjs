// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from 'node:fs';
import path from 'node:path';
const root = process.argv[2] ?? 'openrgb';
const commit = process.argv[3] ?? 'unknown';
const dir = path.join(root, 'Controllers', 'InstantMouseController');
const files = [];
const walk = d => { for (const e of fs.readdirSync(d, { withFileTypes: true })) { const p = path.join(d, e.name); if (e.isDirectory()) walk(p); else if (/\.(cpp|h)$/.test(e.name)) files.push(p); } };
walk(dir);
const source = files.map(f => fs.readFileSync(f, 'utf8')).join('\n');
const defs = new Map([...source.matchAll(/^\s*#define\s+([A-Za-z0-9_]+)\s+(0x[0-9A-Fa-f]+)/gm)].map(m => [m[1], m[2]]));
const value = token => { const v = defs.get(token.trim()) ?? token.trim(); return /^0+[0-9]+$/.test(v) ? String(Number(v)) : v; };
const rows = [];
for (const m of source.matchAll(/REGISTER_HID_DETECTOR_IPU\s*\(\s*"([^"]+)"\s*,[^,]+,\s*([^,\s]+)\s*,\s*([^,\s]+)\s*,\s*([^,\s]+)\s*,\s*([^,\s]+)\s*,\s*([^,\s)]+)/g)) rows.push({ name: m[1], vid: value(m[2]), pid: value(m[3]), interface: value(m[4]), usagePage: value(m[5]), usage: value(m[6]) });
if (rows.length !== 4) throw new Error(`expected 4 detectors, found ${rows.length}`);
const lines = ['// SPDX-License-Identifier: GPL-2.0-or-later', `// Generated from OpenRGB ${commit}.`, 'use crate::table::Detector;', 'pub const DETECTORS: &[Detector] = &[', ...rows.map(x => `Detector { name: ${JSON.stringify(x.name)}, vid: ${x.vid}, pid: ${x.pid}, interface: ${x.interface}, usage_page: ${x.usagePage}, usage: ${x.usage} },`), '];', 'pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&\'static Detector> { DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid && d.interface == i.interface && d.usage_page == i.usage_page && d.usage == i.usage) }'];
fs.writeFileSync('modules/instant-mouse/src/devices_table.rs', lines.join('\n'));
console.log(`generated ${rows.length} Instant mouse detectors`);
