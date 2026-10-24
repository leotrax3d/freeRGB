// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};

#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: i32,
    pub usage_page: u16,
}

pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && d.interface == i.interface
            && d.usage_page == i.usage_page
    });
    Identification {
        supported: d.is_some(),
        model: d.map(|d| d.name.to_owned()),
        kind: d.map(|_| DeviceKind::LightingController),
        reason: d
            .is_none()
            .then(|| "no MSI Mystic Light HID detector for this interface".into()),
    }
}
