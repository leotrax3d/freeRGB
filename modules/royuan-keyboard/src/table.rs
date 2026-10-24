// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
}
pub struct Device {
    pub vid: u16,
    pub pid: u16,
    pub protocol: Protocol,
    pub leds: usize,
}
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Protocol {
    #[default]
    Akko,
    Legacy,
    Skyloong,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid);
    Identification {
        supported: d.is_some(),
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Keyboard),
        reason: d
            .is_none()
            .then(|| "no Royuan keyboard HID detector".into()),
    }
}
pub fn device_count(c: &[freergb_core::Rgb]) -> usize {
    c.len()
}
pub fn device_from_count(n: usize) -> Option<&'static Device> {
    crate::devices_table::DEVICES.iter().find(|d| d.leds == n)
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Device> {
    crate::devices_table::DEVICES
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid)
}
