import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "HP Omen 30L", vid: 0x103C, pid: 0x84FD, interface: -1, usage_page: 0x0000, usage: 0x0000 },',
'];',
];
fs.writeFileSync('modules/hp-omen-30l/src/devices_table.rs', out.join('\n'));
console.log('generated 1 HP Omen 30L detector');
