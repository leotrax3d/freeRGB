// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470a7cba56af60989e54ba910d5ac66a3ac.
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
        interface: 2,
        usage_page: 0xFFFF,
        usage: 0x01,
    },
    Detector {
        name: "Mountain Everest 60",
        vid: 0x3282,
        pid: 0x0006,
        interface: 2,
        usage_page: 0xFFFF,
        usage: 0x01,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
