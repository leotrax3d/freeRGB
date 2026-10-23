// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    /// "full" (124 LEDs, fragmented 0F 03 request), "mini" (106 LEDs, same),
    /// "65" (special 0F 15 direct packet).
    pub protocol: &'static str,
    /// LED total from FnaticStreakController.cpp GetLEDCount / 65 path.
    pub leds: usize,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid && i32::from(d.interface) == i.interface);
    Identification {
        // Detector inventory is not a protocol implementation: Direct mode needs
        // the fragmented SendRequest framing plus keepalive (full/mini) or the
        // Streak65 0F 15 packet with LED count from the device config.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Keyboard),
        reason: Some(if d.is_some() {
            "Fnatic Streak detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no Fnatic Streak HID detector for this interface".into()
        }),
    }
}
