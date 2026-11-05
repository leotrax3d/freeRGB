// SPDX-License-Identifier: GPL-2.0-or-later
use crate::table::Detector;
pub const DETECTORS: &[Detector] = &[Detector {
    name: "Acer Nitro HID LED",
    vid: 0x0CF2,
    pid: 0x5130,
    interface: -1,
    usage_page: 0xFF5A,
    usage: 0x0001,
}];
