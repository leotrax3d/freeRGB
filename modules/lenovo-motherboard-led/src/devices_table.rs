// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "Lenovo Motherboard",
    vid: 0x17EF,
    pid: 0xC955,
    interface: -1,
    usage_page: 0xFF89,
    usage: 0x00CC,
}];
