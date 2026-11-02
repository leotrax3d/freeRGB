// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[
    Detector {
        name: "Advanced GTA 250 USB Gaming Mouse",
        vid: 0x30FA,
        pid: 0x1030,
        interface: 1,
        usage_page: 0xFF01,
        usage: 0x01,
    },
    Detector {
        name: "Anko KM43243952 USB Gaming Mouse",
        vid: 0x30FA,
        pid: 0x1440,
        interface: 1,
        usage_page: 0xFF01,
        usage: 0x01,
    },
    Detector {
        name: "Anko KM43277483 USB Gaming Mouse",
        vid: 0x30FA,
        pid: 0x1540,
        interface: 1,
        usage_page: 0xFF01,
        usage: 0x01,
    },
    Detector {
        name: "AntEsports GM600 USB Gaming Mouse",
        vid: 0x30FA,
        pid: 0x1040,
        interface: 1,
        usage_page: 0xFF01,
        usage: 1,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && d.interface == i.interface
            && d.usage_page == i.usage_page
            && d.usage == i.usage
    })
}
