// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use super::table::Detector;
pub static DETECTORS: &[Detector] = &[
    Detector {
        name: "Sony DualShock 4",
        vid: 0x054C,
        pid: 0x05C4,
        dualsense: false,
    },
    Detector {
        name: "Sony DualShock 4",
        vid: 0x054C,
        pid: 0x09CC,
        dualsense: false,
    },
    Detector {
        name: "Sony DualShock 4",
        vid: 0x054C,
        pid: 0x0BA0,
        dualsense: false,
    },
    Detector {
        name: "Sony DualSense",
        vid: 0x054C,
        pid: 0x0CE6,
        dualsense: true,
    },
    Detector {
        name: "Sony DualSense Edge",
        vid: 0x054C,
        pid: 0x0DF2,
        dualsense: true,
    },
];
pub fn device(i: &freergb_module_api::InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
