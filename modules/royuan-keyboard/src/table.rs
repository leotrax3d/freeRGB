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
    /// "akko" (profile protocol), "legacy" (Epomaker/Attack Shark) or
    /// "skyloong" (GK68HE Pro controller).
    pub protocol: &'static str,
}
fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid && i32::from(d.interface) == i.interface)
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    match detector(i) {
        Some(d) => Identification {
            // Detector inventory is not a protocol implementation: the audited
            // branch version used output writes with a wrong packet layout; all
            // three upstream paths send feature reports (mode packets, profile
            // state and per-color sets with per-protocol checksum types).
            supported: false,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Keyboard),
            reason: Some(format!(
                "Royuan detector {} is inventoried but its {} protocol is not ported",
                d.name, d.protocol
            )),
        },
        None => Identification {
            supported: false,
            model: None,
            kind: None,
            reason: Some("no Royuan HID detector for this interface".into()),
        },
    }
}
