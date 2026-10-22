// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: i32,
    pub usage_page: u16,
    pub usage: u16,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = device(i);
    Identification {
        supported: d.is_some(),
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Mouse),
        reason: d.is_none().then(|| "no Instant mouse HID detector".into()),
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    crate::devices_table::device(i)
}
