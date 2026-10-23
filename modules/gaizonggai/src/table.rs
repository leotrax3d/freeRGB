// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    /// Fixed zone LED counts from the upstream SetupZones; 0 marks a
    /// device-derived count (LightBoard/RGB HUB rows need flash/channel reads).
    pub leds: usize,
}
fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid && i32::from(d.interface) == i.interface)
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = detector(i);
    Identification {
        // Detector inventory is not a protocol implementation: the audited branch
        // version sent generic packets (sequential 20/21-byte chunks from packet
        // command 0x10) that match none of the upstream per-PID layouts, and the
        // LightBoard/RGB HUB rows need flash data and channel-length reads.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Keyboard),
        reason: Some(if d.is_some() {
            "GaiZhongGai detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no GaiZhongGai HID detector for this interface".into()
        }),
    }
}
