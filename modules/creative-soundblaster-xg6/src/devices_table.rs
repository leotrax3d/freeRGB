// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "Creative SoundBlasterX G6",
    vid: 0x041E,
    pid: 0x3256,
    interface: 4,
    usage_page: 0x0000,
    usage: 0x0000,
}];
