// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::{DETECTORS, DEVICES};
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
    let dev = DEVICES.iter().find(|d| d.pid == i.pid);
    let supported = d.is_some()
        && dev
            .is_some_and(|d| d.version == 1 || d.version == 2 || d.version == 3 || d.version == 4);
    Identification {
        supported,
        model: d.map(|x| x.name.into()),
        kind: supported.then_some(DeviceKind::LightingController),
        reason: (!supported).then(|| {
            if d.is_some() {
                "DeepRGB detector is inventoried but only version-1 Direct-mode path is pending"
                    .into()
            } else {
                "no DeepRGB HID detector for this interface".into()
            }
        }),
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Device> {
    (crate::devices_table::DEVICES
        .iter()
        .find(|d| d.pid == i.pid))
    .filter(|d| d.version == 1)
}
