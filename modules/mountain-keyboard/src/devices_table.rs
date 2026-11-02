// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[
    Detector {
        name: "Mountain Everest",
        vid: 0x3282,
        pid: 0x0001,
        interface: 3,
        usage_page: 0xFF00,
        usage: 0x01,
    },
    Detector {
        name: "Mountain Everest 60",
        vid: 0x3282,
        pid: 0x0005,
        interface: MOUNTAIN60_EVEREST_60_INTERFACE,
        usage_page: 0xFFFF,
        usage: 0x01,
    },
    Detector {
        name: "Mountain Everest 60",
        vid: 0x3282,
        pid: 0x0006,
        interface: MOUNTAIN60_EVEREST_60_INTERFACE,
        usage_page: 0xFFFF,
        usage: 0x01,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
