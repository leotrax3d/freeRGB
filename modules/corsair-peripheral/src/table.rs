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
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid);
    Identification {
        // Detector inventory is not a protocol implementation: the audit found no
        // fully ported Direct-mode path for any of the three handlers. The generic
        // handler needs firmware-info device typing, typed LightingControl and the
        // STREAM 0x7F keyboard path; K55 PRO XT needs the 7-packet key-bank stream;
        // K65 Mini needs the LED position map, 64-LED layout and keepalive.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "Corsair peripheral detector is inventoried but its controller-specific protocol is not ported"
                .into()
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
