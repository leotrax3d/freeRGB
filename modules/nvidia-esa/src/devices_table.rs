// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "Nvidia ESA - Dell XPS 730x",
    vid: 0x0955,
    pid: 0x000A,
    interface: -1,
    usage_page: 0xFFDE,
    usage: 0x0002,
}];
