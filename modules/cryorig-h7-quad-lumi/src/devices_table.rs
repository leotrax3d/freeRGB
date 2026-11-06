// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "CRYORIG H7 Quad Lumi",
    vid: 0x1E71,
    pid: 0x1712,
    interface: -1,
    usage_page: 0x0000,
    usage: 0x0000,
}];
