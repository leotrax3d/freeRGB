// SPDX-License-Identifier: MIT OR Apache-2.0
import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
`// Generated from OpenRGB ${commit}.`,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "Dream Cheeky Webmail Notifier", vid: 0x1D34, pid: 0x0004, interface: 0, usage_page: 0, usage: 0 },',
'];',
];
fs.writeFileSync('modules/dream-cheeky/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Dream Cheeky detector');