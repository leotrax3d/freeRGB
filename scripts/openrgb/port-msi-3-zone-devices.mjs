import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
`// Generated from OpenRGB ${commit}.`,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "MSI 3-Zone Laptop", vid: 0x1770, pid: 0xFF00, interface: 0, usage_page: 0, usage: 0 },',
'];',
];
fs.writeFileSync('modules/msi-3-zone/src/devices_table.rs', out.join('\n'));
console.log('generated 1 MSI 3-Zone detector');