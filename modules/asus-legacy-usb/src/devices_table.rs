// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[
    Detector {
        name: "ASUS Cerberus Mech",
        vid: 0x195D,
        pid: 0x2047,
        interface: 1,
        usage_page: 0xFF01,
        usage: 1,
    },
    Detector {
        name: "ASUS Sagaris GK1100",
        vid: 0x0B05,
        pid: 0x1835,
        interface: 1,
        usage_page: 0xFF02,
        usage: 2,
    },
    Detector {
        name: "ASUS ROG Strix Claw",
        vid: 0x195D,
        pid: 0x1016,
        interface: 0,
        usage_page: 0xFF01,
        usage: 1,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
