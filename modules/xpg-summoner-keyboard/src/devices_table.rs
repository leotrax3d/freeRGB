// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "XPG Summoner Gaming Keyboard",
    vid: 0x125F,
    pid: 0x9418,
    interface: 2,
    usage_page: 0xFF01,
    usage: 0x0001,
}];
