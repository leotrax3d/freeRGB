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
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid);
    Identification {
        // Three different upstream controllers are only inventoried here;
        // the current Rust implementation is not a complete legacy port.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Mouse),
        reason: Some(if d.is_some() {
            "ASUS legacy detector is inventoried but its complete keyboard/mouse protocol is not ported".into()
        } else {
            "no ASUS legacy HID detector".into()
        }),
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    crate::devices_table::device(i)
}
