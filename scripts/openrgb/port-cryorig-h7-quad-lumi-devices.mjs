import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "CRYORIG H7 Quad Lumi", vid: 0x1E71, pid: 0x1712, interface: -1, usage_page: 0x0000, usage: 0x0000 },',
'];',
];
fs.writeFileSync('modules/cryorig-h7-quad-lumi/src/devices_table.rs', out.join('\n'));
console.log('generated 1 Cryorig detector');
