// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[
    Detector {
        name: "Das Keyboard Q4 RGB",
        vid: 0x24F0,
        pid: 0x2037,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
    Detector {
        name: "Das Keyboard Q5 RGB",
        vid: 0x24F0,
        pid: 0x2020,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
    Detector {
        name: "Das Keyboard Q5S RGB",
        vid: 0x24F0,
        pid: 0x209A,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
