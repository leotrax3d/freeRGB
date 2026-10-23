// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// RGB channel count from the upstream spawn_hue call; 0 marks a detector
    /// routed to a different controller (Kraken X3) that cannot use this protocol.
    pub channels: usize,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    match device(i) {
        Some(d) => Identification {
            supported: true,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::LightingController),
            reason: None,
        },
        None => Identification {
            supported: false,
            model: None,
            kind: None,
            reason: Some("no NZXT HID detector for this VID/PID".into()),
        },
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
