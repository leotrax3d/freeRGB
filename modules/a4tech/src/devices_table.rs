// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470a7cba56af60989e54ba910d5ac66a3ac.
use super::table::{Detector, Device};
pub static DETECTORS: &[Detector] = &[
    Detector {
        name: "Bloody B820R",
        handler: "DetectBloodyB820R",
        vid: 0x09DA,
        pid: 0xFA10,
        protocol: "b820",
        leds: 104,
    },
    Detector {
        name: "Bloody W60 Pro",
        handler: "DetectBloodyMouseControllers",
        vid: 0x09DA,
        pid: 0x37EA,
        protocol: "mouse",
        leds: 3,
    },
    Detector {
        name: "Bloody W70 Max",
        handler: "DetectBloodyMouseControllers",
        vid: 0x09DA,
        pid: 0x79EF,
        protocol: "mouse",
        leds: 3,
    },
    Detector {
        name: "Bloody W90 Max",
        handler: "DetectBloodyMouseControllers",
        vid: 0x09DA,
        pid: 0x3666,
        protocol: "mouse",
        leds: 3,
    },
    Detector {
        name: "Bloody W90 Pro",
        handler: "DetectBloodyMouseControllers",
        vid: 0x09DA,
        pid: 0x39B6,
        protocol: "mouse",
        leds: 3,
    },
    Detector {
        name: "Bloody MP 50RS",
        handler: "DetectBloodyMouseControllers",
        vid: 0x09DA,
        pid: 0xFA60,
        protocol: "mouse",
        leds: 3,
    },
];
pub static DEVICES: &[Device] = &[
    Device {
        vid: 0x09DA,
        pid: 0xFA10,
        protocol: "b820",
        leds: 104,
    },
    Device {
        vid: 0x09DA,
        pid: 0x37EA,
        protocol: "mouse",
        leds: 3,
    },
    Device {
        vid: 0x09DA,
        pid: 0x79EF,
        protocol: "mouse",
        leds: 3,
    },
    Device {
        vid: 0x09DA,
        pid: 0x3666,
        protocol: "mouse",
        leds: 3,
    },
    Device {
        vid: 0x09DA,
        pid: 0x39B6,
        protocol: "mouse",
        leds: 3,
    },
    Device {
        vid: 0x09DA,
        pid: 0xFA60,
        protocol: "mouse",
        leds: 3,
    },
];
