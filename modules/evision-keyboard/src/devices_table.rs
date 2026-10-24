// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470a7cba56af60989e54ba910d5ac66a3ac.
use super::table::{Detector, Device};
pub static DETECTORS: &[Detector] = &[
    Detector {
        name: "EVision Keyboard 0C45:5004",
        handler: "DetectEVisionKeyboards",
        vid: 0x0C45,
        pid: 0x5004,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 0C45:5104",
        handler: "DetectEVisionKeyboards",
        vid: 0x0C45,
        pid: 0x5104,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 0C45:5204",
        handler: "DetectEVisionKeyboards",
        vid: 0x0C45,
        pid: 0x5204,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 0C45:652F",
        handler: "DetectEVisionKeyboards",
        vid: 0x0C45,
        pid: 0x652F,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 0C45:7672",
        handler: "DetectEVisionKeyboards",
        vid: 0x0C45,
        pid: 0x7672,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 0C45:7698",
        handler: "DetectEVisionKeyboards",
        vid: 0x0C45,
        pid: 0x7698,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 0C45:8520",
        handler: "DetectEVisionKeyboards",
        vid: 0x0C45,
        pid: 0x8520,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 320F:5000",
        handler: "DetectEVisionKeyboards",
        vid: 0x320F,
        pid: 0x5000,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 320F:502A",
        handler: "DetectEVisionKeyboards",
        vid: 0x320F,
        pid: 0x502A,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 320F:505B",
        handler: "DetectEVisionKeyboards",
        vid: 0x320F,
        pid: 0x505B,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 320F:5078",
        handler: "DetectEVisionKeyboards",
        vid: 0x320F,
        pid: 0x5078,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 320F:5064",
        handler: "DetectEVisionKeyboards",
        vid: 0x320F,
        pid: 0x5064,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 320F:5084",
        handler: "DetectEVisionKeyboards",
        vid: 0x320F,
        pid: 0x5084,
        v2: false,
    },
    Detector {
        name: "EVision Keyboard 320F:5041",
        handler: "DetectEVisionV2Keyboards",
        vid: 0x320F,
        pid: 0x5041,
        v2: true,
    },
    Detector {
        name: "Endorfy Omnis",
        handler: "DetectEndorfyKeyboards",
        vid: 0x3299,
        pid: 0x0012,
        v2: true,
    },
];
pub static DEVICES: &[Device] = &[
    Device {
        vid: 0x0C45,
        pid: 0x5004,
        v2: false,
    },
    Device {
        vid: 0x0C45,
        pid: 0x5104,
        v2: false,
    },
    Device {
        vid: 0x0C45,
        pid: 0x5204,
        v2: false,
    },
    Device {
        vid: 0x0C45,
        pid: 0x652F,
        v2: false,
    },
    Device {
        vid: 0x0C45,
        pid: 0x7672,
        v2: false,
    },
    Device {
        vid: 0x0C45,
        pid: 0x7698,
        v2: false,
    },
    Device {
        vid: 0x0C45,
        pid: 0x8520,
        v2: false,
    },
    Device {
        vid: 0x320F,
        pid: 0x5000,
        v2: false,
    },
    Device {
        vid: 0x320F,
        pid: 0x502A,
        v2: false,
    },
    Device {
        vid: 0x320F,
        pid: 0x505B,
        v2: false,
    },
    Device {
        vid: 0x320F,
        pid: 0x5078,
        v2: false,
    },
    Device {
        vid: 0x320F,
        pid: 0x5064,
        v2: false,
    },
    Device {
        vid: 0x320F,
        pid: 0x5084,
        v2: false,
    },
    Device {
        vid: 0x320F,
        pid: 0x5041,
        v2: true,
    },
    Device {
        vid: 0x3299,
        pid: 0x0012,
        v2: true,
    },
];
