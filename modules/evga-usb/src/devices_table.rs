// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470a7cba56af60989e54ba910d5ac66a3ac.
use super::table::{Detector, Device};
pub static DETECTORS: &[Detector] = &[
    Detector {
        name: "EVGA Z15 Keyboard",
        handler: "DetectEVGAKeyboardControllers",
        vid: 0x3842,
        pid: 0x260E,
        protocol: "keyboard",
        leds: 104,
    },
    Detector {
        name: "EVGA Z15 Keyboard",
        handler: "DetectEVGAKeyboardControllers",
        vid: 0x3842,
        pid: 0x2608,
        protocol: "keyboard",
        leds: 104,
    },
    Detector {
        name: "EVGA Z20 Keyboard",
        handler: "DetectEVGAKeyboardControllers",
        vid: 0x3842,
        pid: 0x260A,
        protocol: "keyboard",
        leds: 104,
    },
    Detector {
        name: "EVGA Z20 Keyboard",
        handler: "DetectEVGAKeyboardControllers",
        vid: 0x3842,
        pid: 0x2610,
        protocol: "keyboard",
        leds: 104,
    },
    Detector {
        name: "EVGA X20 Gaming Mouse",
        handler: "DetectWiredEVGAMouse",
        vid: 0x3842,
        pid: 0x2420,
        protocol: "mouse",
        leds: 3,
    },
    Detector {
        name: "EVGA X20 USB Receiver",
        handler: "DetectWirelessEVGAMouse",
        vid: 0x3842,
        pid: 0x2402,
        protocol: "mouse",
        leds: 3,
    },
    Detector {
        name: "EVGA X12 Gaming Mouse",
        handler: "DetectEVGAX12",
        vid: 0x3842,
        pid: 0x2422,
        protocol: "x12",
        leds: 3,
    },
];
pub static DEVICES: &[Device] = &[
    Device {
        vid: 0x3842,
        pid: 0x260E,
        protocol: "keyboard",
        leds: 104,
    },
    Device {
        vid: 0x3842,
        pid: 0x2608,
        protocol: "keyboard",
        leds: 104,
    },
    Device {
        vid: 0x3842,
        pid: 0x260A,
        protocol: "keyboard",
        leds: 104,
    },
    Device {
        vid: 0x3842,
        pid: 0x2610,
        protocol: "keyboard",
        leds: 104,
    },
    Device {
        vid: 0x3842,
        pid: 0x2420,
        protocol: "mouse",
        leds: 3,
    },
    Device {
        vid: 0x3842,
        pid: 0x2402,
        protocol: "mouse",
        leds: 3,
    },
    Device {
        vid: 0x3842,
        pid: 0x2422,
        protocol: "x12",
        leds: 3,
    },
];
