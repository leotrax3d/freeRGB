import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "Lenovo Motherboard", vid: 0x17EF, pid: 0xC955, interface: -1, usage_page: 0xFF89, usage: 0x00CC },',
'];',
];
fs.writeFileSync('modules/lenovo-motherboard-led/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Lenovo Motherboard detector');
