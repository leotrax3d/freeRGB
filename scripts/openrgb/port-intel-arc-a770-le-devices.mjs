import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "Intel Arc A770 Limited Edition", vid: 0x2516, pid: 0x01B5, interface: 1, usage_page: 0xFF00, usage: 0x0000 },',
'];',
];
fs.writeFileSync('modules/intel-arc-a770-le/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Intel Arc A770 LE detector');
