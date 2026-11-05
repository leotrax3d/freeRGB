import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "Creative SoundBlasterX G6", vid: 0x041E, pid: 0x3256, interface: 4, usage_page: 0x0000, usage: 0x0000 },',
'];',
];
fs.writeFileSync('modules/creative-soundblaster-xg6/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Creative SoundBlasterX G6 detector');
