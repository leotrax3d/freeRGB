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
            && (d.interface < 0 || (d.interface == i.interface && d.usage_page == i.usage_page))
    });
    Identification {
        // The detector list covers separate desktop and laptop upstream
        // paths; it is not a complete protocol implementation.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::LightingController),
        reason: Some(if d.is_some() {
            "ASUS Aura Core detector is inventoried but its complete protocol is not ported".into()
        } else {
            "no ASUS Aura Core HID detector for this interface".into()
        }),
    }
}
