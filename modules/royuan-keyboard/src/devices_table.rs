// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470.
use super::table::{Detector, Device, Protocol};
pub static DETECTORS: &[Detector] = &[
    Detector {
        name: "Akko 3068B Plus",
        vid: 0x3151,
        pid: 0x4003,
    },
    Detector {
        name: "Skyloong GK68HE Pro",
        vid: 0x3151,
        pid: 0x5029,
    },
    Detector {
        name: "Epomaker TH80 Pro (USB Cable)",
        vid: 0x3151,
        pid: 0x4010,
    },
    Detector {
        name: "Epomaker TH80 Pro (USB Dongle)",
        vid: 0x3151,
        pid: 0x4011,
    },
    Detector {
        name: "Attack Shark K86 (USB Cable)",
        vid: 0x3151,
        pid: 0x4015,
    },
];
pub static DEVICES: &[Device] = &[
    Device {
        vid: 0x3151,
        pid: 0x4003,
        protocol: Protocol::Akko,
        leds: 1,
    },
    Device {
        vid: 0x3151,
        pid: 0x5029,
        protocol: Protocol::Skyloong,
        leds: 67,
    },
    Device {
        vid: 0x3151,
        pid: 0x4010,
        protocol: Protocol::Legacy,
        leds: 1,
    },
    Device {
        vid: 0x3151,
        pid: 0x4011,
        protocol: Protocol::Legacy,
        leds: 1,
    },
    Device {
        vid: 0x3151,
        pid: 0x4015,
        protocol: Protocol::Legacy,
        leds: 1,
    },
];
