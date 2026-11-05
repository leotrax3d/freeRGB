import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "Acer Nitro HID Keyboard", vid: 0x0CF2, pid: 0x5130, interface: -1, usage_page: 0xFF5A, usage: 0x0001 },',
'];',
];
fs.writeFileSync('modules/acer-nitro-hid-keyboard/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Acer Nitro HID Keyboard detector');
