// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[
    Detector {
        name: "Fnatic Streak",
        vid: 0x2F0E,
        pid: 0x0101,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
    Detector {
        name: "Fnatic miniStreak",
        vid: 0x2F0E,
        pid: 0x0102,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
    Detector {
        name: "Fnatic Streak65",
        vid: 0x2F0E,
        pid: 0x0105,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
