// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};

#[derive(Debug, Clone, PartialEq)]
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    pub usage_page: u16,
    pub usage: u16,
}

pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = device(i);
    Identification {
        supported: d.is_some(),
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Keyboard),
        reason: d
            .is_none()
            .then(|| "no Roccat Horde Aimo HID detector for this interface".into()),
    }
}

pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && i32::from(d.interface) == i.interface
            && d.usage_page == i.usage_page
            && d.usage == i.usage
    })
}
