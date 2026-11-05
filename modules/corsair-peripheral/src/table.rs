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
pub fn supported_pid(pid: u16) -> bool {
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
            | 0x1b3f
            | 0x1ba5
            | 0x1b13
            | 0x1b17
            | 0x1b37
            | 0x1b39
            | 0x1b36
            | 0x1bc6
            | 0x1b82
            | 0x1b20
            | 0x1b44
    )
}
fn kind_for(pid: u16) -> DeviceKind {
    match pid {
        0x1b3b | 0x0a34 => DeviceKind::LightingController,
        0x1b3f | 0x1ba5 | 0x1b13 | 0x1b17 | 0x1b37 | 0x1b39 | 0x1b36 | 0x1bc6 | 0x1b82 | 0x1b20
        | 0x1b44 => DeviceKind::Keyboard,
        _ => DeviceKind::Mouse,
    }
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid);
    if let Some(d) = d {
        if supported_pid(d.pid) {
            return Identification {
                supported: true,
                model: Some(d.name.into()),
                kind: Some(kind_for(d.pid)),
                reason: None,
            };
        }
    }
    Identification {
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "Corsair peripheral detector is inventoried; K55/K95/K70 Pro/K68 keyboard paths remain unported".into()
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
