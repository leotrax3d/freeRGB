// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "MSI Optix MAG274QRF",
    vid: 0x1462,
    pid: 0x3FA4,
    interface: 0,
    usage_page: 0xFF00,
    usage: 0x0001,
}];
