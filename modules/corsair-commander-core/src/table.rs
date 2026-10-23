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
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid);
    Identification {
        // Detector inventory is not a protocol implementation: the protocol needs
        // the version query, packet-size negotiation, SetFanMode/SetLedAmount
        // writes with device reads and config-driven LED counts; the audited
        // branch version sent a fabricated 97-byte packet instead.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::LightingController),
        reason: Some(if d.is_some() {
            "Corsair Commander Core detector is inventoried but its protocol is not ported".into()
        } else {
            "no Corsair Commander Core HID detector for this VID/PID".into()
        }),
    }
}
