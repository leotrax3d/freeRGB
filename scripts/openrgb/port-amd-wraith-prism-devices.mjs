import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "AMD Wraith Prism", vid: 0x2516, pid: 0x0051, interface: 1, usage_page: 0xFF00, usage: 0x0000 },',
'];',
];
fs.writeFileSync('modules/amd-wraith-prism/src/devices_table.rs', out.join('\n'));
console.log('generated 1 AMD Wraith Prism detector');
