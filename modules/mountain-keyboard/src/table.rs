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
    pub usage: u16,
}
fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && d.interface == i.interface
            && d.usage_page == i.usage_page
            && d.usage == i.usage
    })
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = detector(i);
    Identification {
        supported: false,
        model: d.map(|x| x.name.to_string()),
        kind: d.map(|_| DeviceKind::Keyboard),
        reason: Some(if d.is_some() {
            "Mountain keyboard protocol is not ported; only the HID detector is inventoried"
                .to_string()
        } else {
            "no Mountain keyboard HID detector for this interface".to_string()
        }),
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    detector(i)
}
