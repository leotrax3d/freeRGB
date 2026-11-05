// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "HP Omen 30L",
    vid: 0x103C,
    pid: 0x84FD,
    interface: -1,
    usage_page: 0x0000,
    usage: 0x0000,
}];
