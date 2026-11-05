import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
`// Generated from OpenRGB ${commit}.`,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "Tecknet M008", vid: 0x04D9, pid: 0xFC05, interface: 0, usage_page: 0xFFA0, usage: 0x01 },',
'];',
];
fs.writeFileSync('modules/tecknet/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Tecknet detector');