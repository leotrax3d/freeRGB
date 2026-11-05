// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "AMD Wraith Prism",
    vid: 0x2516,
    pid: 0x0051,
    interface: 1,
    usage_page: 0xFF00,
    usage: 0x0000,
}];
