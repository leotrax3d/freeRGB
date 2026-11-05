import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "Nvidia ESA - Dell XPS 730x", vid: 0x0955, pid: 0x000A, interface: -1, usage_page: 0xFFDE, usage: 0x0002 },',
'];',
];
fs.writeFileSync('modules/nvidia-esa/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Nvidia ESA detector');