// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};

#[derive(Debug, Clone, PartialEq)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// Upstream registers this detector with an interface constraint
    /// (`REGISTER_HID_DETECTOR_I`, interface 4).
    pub interface: u8,
    /// Unused by the upstream detector row.
    pub usage_page: u16,
    pub usage: u16,
}

pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid && i32::from(d.interface) == i.interface);
    Identification {
        supported: d.is_some(),
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Keypad),
        reason: d
            .is_none()
            .then(|| "no Turtle Beach KP7 HID detector for this interface".into()),
    }
}

pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid && i32::from(d.interface) == i.interface)
}
