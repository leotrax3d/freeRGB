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
    pub usage_page: u16,
    pub usage: u16,
    /// "keyboard", "mouse", "wireless" (receiver) or "x12".
    pub protocol: &'static str,
}
fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && i32::from(d.interface) == i.interface
            && d.usage_page == i.usage_page
            && d.usage == i.usage
    })
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = detector(i);
    Identification {
        // Detector inventory is not a protocol implementation: the audited branch
        // version fabricated 65/40/136-byte packets; upstream needs the keyboard
        // packet_map offsets with multiple controller IDs, mouse SetLed with
        // feature-report queries and delays, and the X12 read/state machine.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "EVGA detector is inventoried but its controller-specific protocol is not ported".into()
        } else {
            "no EVGA HID detector for this interface".into()
        }),
    }
}
