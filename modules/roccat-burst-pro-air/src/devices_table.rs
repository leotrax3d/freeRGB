// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "Roccat Burst Pro Air",
    vid: 0x1E7D,
    pid: 0x2CA6,
    interface: 0,
    usage_page: 0x0001,
    usage: 0x0002,
}];
