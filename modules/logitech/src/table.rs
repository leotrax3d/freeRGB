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
/// Handlers whose Direct-mode path is fully ported to Rust and audited
/// against the upstream OpenRGB controllers. Everything else stays
/// inventoried but unsupported.
pub const SUPPORTED_HANDLERS: &[&str] = &[
    "DetectLogitechKeyboardG213",
    "DetectLogitechKeyboardG915",
    "DetectLogitechKeyboardG915Wired",
    "DetectLogitechMouseG203L",
    "DetectLogitechMouseG600",
    "DetectLogitechG933",
    "DetectLogitechX56",
];
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid);
    match d {
        Some(d) if SUPPORTED_HANDLERS.contains(&d.handler) => Identification {
            supported: true,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Accessory),
            reason: None,
        },
        Some(d) => Identification {
            supported: false,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Accessory),
            reason: Some(format!(
                "Logitech detector {} is inventoried but its direct-mode protocol path is not fully ported (audit)",
                d.handler
            )),
        },
        None => Identification {
            supported: false,
            model: None,
            kind: None,
            reason: Some("no Logitech HID detector for this VID/PID".into()),
        },
    }
}
pub fn handler(i: &InterfaceInfo) -> Option<&'static str> {
    DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid)
        .map(|d| d.handler)
}
