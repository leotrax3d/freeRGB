import fs from 'node:fs';
const commit=process.argv[3]??'unknown';
const out=[
'// SPDX-License-Identifier: GPL-2.0-or-later',
// Generated from OpenRGB .,
'use crate::table::Detector;',
'pub const DETECTORS: &[Detector] = &[',
'    Detector { name: "MSI Optix MAG274QRF", vid: 0x1462, pid: 0x3FA4, interface: 0, usage_page: 0xFF00, usage: 0x0001 },',
'];',
];
fs.writeFileSync('modules/msi-optix/src/devices_table.rs', out.join('\n'));
console.log('generated 1 MSI Optix detector');
