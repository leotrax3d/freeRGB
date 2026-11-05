import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "HyperX QuadCast 2 S", vid: 0x03F0, pid: 0x02B5, interface: 1, usage_page: 0xFF13, usage: 0xFF00 },',
'];',
];
fs.writeFileSync('modules/hyperx-microphone-v2/src/devices_table.rs', out.join('\n'));
console.log('generated 1 HyperX QuadCast 2 S detector');
