// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "Thermaltake Poseidon Z RGB",
    vid: 0x264A,
    pid: 0x3006,
    interface: 1,
    usage_page: 0xFF01,
    usage: 0x0000,
}];
