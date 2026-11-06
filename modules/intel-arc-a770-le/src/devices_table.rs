// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "Intel Arc A770 Limited Edition",
    vid: 0x2516,
    pid: 0x01B5,
    interface: 1,
    usage_page: 0xFF00,
    usage: 0x0000,
}];
