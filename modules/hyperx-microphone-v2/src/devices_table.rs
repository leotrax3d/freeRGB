// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "HyperX QuadCast 2 S",
    vid: 0x03F0,
    pid: 0x02B5,
    interface: 1,
    usage_page: 0xFF13,
    usage: 0xFF00,
}];
