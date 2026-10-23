// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    pub usage_page: u16,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = device(i);
    Identification {
        supported: d.is_some(),
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Keyboard),
        reason: d
            .is_none()
            .then(|| "no Cherry keyboard HID detector for this interface".into()),
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    crate::devices_table::DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && d.interface as i32 == i.interface
            && d.usage_page == i.usage_page
    })
}
