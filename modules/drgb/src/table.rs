// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
}
#[derive(Debug)]
pub struct Device {
    pub pid: u16,
    pub channels: usize,
    pub leds_per_channel: usize,
    pub version: u8,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid);
    Identification {
        supported: d.is_some(),
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: d
            .is_none()
            .then(|| "no DRGB HID detector for this VID/PID".into()),
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Device> {
    crate::devices_table::DEVICES
        .iter()
        .find(|d| d.pid == i.pid)
}
