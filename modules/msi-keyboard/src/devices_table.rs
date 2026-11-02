// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[
    Detector {
        name: "MSI Keyboard MS_1565",
        vid: 0x1462,
        pid: 0x1601,
        interface: -1,
        usage_page: 0x00FF,
        usage: 0x01,
    },
    Detector {
        name: "MSI Mystic Light MS_1562",
        vid: 0x1462,
        pid: 0x1562,
        interface: -1,
        usage_page: 0x00FF,
        usage: 0x01,
    },
    Detector {
        name: "MSI Mystic Light MS_1563",
        vid: 0x1462,
        pid: 0x1563,
        interface: -1,
        usage_page: 0x00FF,
        usage: 0x01,
    },
    Detector {
        name: "MSI Mystic Light MS_1564",
        vid: 0x1462,
        pid: 0x1564,
        interface: -1,
        usage_page: 0x00FF,
        usage: 0x01,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
