// SPDX-License-Identifier: GPL-2.0-or-later
// Generated from OpenRGB bbc77470a7cba56af60989e54ba910d5ac66a3ac.
use super::table::Detector;

pub static DETECTORS: &[Detector] = &[
    Detector {
        name: "Akko 3068B Plus",
        handler: "DetectRoyuanAkkoKeyboardControllers",
        vid: 0x3151,
        pid: 0x4003,
        interface: 0,
        protocol: "akko",
    },
    Detector {
        name: "Skyloong GK68HE Pro",
        handler: "DetectRoyuanGK68HEProKeyboardControllers",
        vid: 0x3151,
        pid: 0x5029,
        interface: 2,
        protocol: "skyloong",
    },
    Detector {
        name: "Epomaker TH80 Pro (USB Cable)",
        handler: "DetectRoyuanLegacyKeyboardControllers",
        vid: 0x3151,
        pid: 0x4010,
        interface: 2,
        protocol: "legacy",
    },
    Detector {
        name: "Epomaker TH80 Pro (USB Dongle)",
        handler: "DetectRoyuanLegacyKeyboardControllers",
        vid: 0x3151,
        pid: 0x4011,
        interface: 2,
        protocol: "legacy",
    },
    Detector {
        name: "Attack Shark K86 (USB Cable)",
        handler: "DetectRoyuanLegacyKeyboardControllers",
        vid: 0x3151,
        pid: 0x4015,
        interface: 2,
        protocol: "legacy",
    },
];
