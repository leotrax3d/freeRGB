// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    pub usage_page: u16,
    pub usage: u16,
    /// true for the Keyboard handlers; only those route a direct-mode controller
    /// (upstream RGBController_GigabyteAorusLaptop.cpp adds the Direct mode only
    /// for GIGABYTE_AORUS_LAPTOP_KEYBOARD_TYPE).
    pub keyboard: bool,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && i32::from(d.interface) == i.interface
            && d.usage_page == i.usage_page
            && d.usage == i.usage
    });
    match d {
        Some(d) if d.keyboard => Identification {
            supported: true,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Keyboard),
            reason: None,
        },
        Some(d) => Identification {
            supported: false,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Keyboard),
            reason: Some(
                "backlight controllers have no direct-mode path in upstream and are not supported here"
                    .into(),
            ),
        },
        None => Identification {
            supported: false,
            model: None,
            kind: None,
            reason: Some("no Gigabyte Aorus laptop HID detector for this interface".into()),
        },
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && i32::from(d.interface) == i.interface
            && d.usage_page == i.usage_page
            && d.usage == i.usage
    })
}
