// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
`// Generated from OpenRGB ${commit}.`,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "CLEVO Lightbar", vid: 0x048D, pid: 0x7001, interface: 0, usage_page: 0xFF03, usage: 0x02 },',
'];',
];
fs.writeFileSync('modules/clevo-lightbar/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Clevo Lightbar detector');