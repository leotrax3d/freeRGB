// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[
    Detector {
        name: "MSI Laptop Keyboard",
        vid: 0x1038,
        pid: 0x1122,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
    Detector {
        name: "MSI Laptop Lightbar",
        vid: 0x1038,
        pid: 0x1161,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
    Detector {
        name: "MSI GE68HX Keyboard",
        vid: 0x1038,
        pid: 0x113a,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
    Detector {
        name: "MSI GE68HX Lightbar",
        vid: 0x1038,
        pid: 0x114b,
        interface: -1,
        usage_page: 0,
        usage: 0,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
