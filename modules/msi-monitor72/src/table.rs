// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};

#[derive(Debug, Clone, PartialEq)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: i32,
    pub usage_page: u16,
    pub usage: u16,
}

fn match_row(i: &InterfaceInfo) -> bool {
    DETECTORS.iter().any(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && d.interface == i.interface
            && d.usage_page == i.usage_page
            && d.usage == i.usage
    })
}

pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = match_row(i);
    Identification {
        supported,
        model: supported.then(|| DETECTORS[0].name.into()),
        kind: supported.then_some(DeviceKind::Accessory),
        reason: (!supported)
            .then(|| "no MSI monitor (gaming controller) HID detector for this interface".into()),
    }
}

pub fn device(i: &InterfaceInfo) -> bool {
    match_row(i)
}
