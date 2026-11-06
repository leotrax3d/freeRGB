import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "Roccat Burst Pro Air", vid: 0x1E7D, pid: 0x2CA6, interface: 0, usage_page: 0x0001, usage: 0x0002 },',
'];',
];
fs.writeFileSync('modules/roccat-burst-pro-air/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Roccat Burst Pro Air detector');
