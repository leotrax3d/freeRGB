import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "XPG Summoner Gaming Keyboard", vid: 0x125F, pid: 0x9418, interface: 2, usage_page: 0xFF01, usage: 0x0001 },',
'];',
];
fs.writeFileSync('modules/xpg-summoner-keyboard/src/devices_table.rs', out.join('\n'));
console.log('generated 1 XPG Summoner detector');
