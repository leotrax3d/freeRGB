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
fn portable_mouse_pid(pid: u16) -> bool {
    matches!(
        pid,
        0x1b34
            | 0x1b74
            | 0x1b3c
            | 0x1b75
            | 0x1b5d
            | 0x1b12
            | 0x1b2e
            | 0x1b5a
            | 0x1b5c
            | 0x1b1e
            | 0x1b3e
            | 0x1b8b
            | 0x1b2f
            | 0x1b32
            | 0x1b3b
            | 0x0a34
    )
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid);
    if let Some(d) = d {
        if portable_mouse_pid(d.pid) {
            return Identification {
                supported: true,
                model: Some(d.name.into()),
                kind: Some(if d.pid == 0x1b3b || d.pid == 0x0a34 {
                    DeviceKind::LightingController
                } else {
                    DeviceKind::Mouse
                }),
                reason: None,
            };
        }
    }
    Identification {
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "Corsair peripheral detector is inventoried; keyboards remain unported".into()
        } else {
            "no Corsair peripheral HID detector for this VID/PID".into()
        }),
    }
}
pub fn handler(i: &InterfaceInfo) -> Option<&'static str> {
    DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid)
        .map(|d| d.handler)
}
