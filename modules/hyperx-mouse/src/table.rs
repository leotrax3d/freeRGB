// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// 255 = HID_INTERFACE_ANY like the upstream detector.
    pub interface: u8,
    pub usage_page: u16,
    pub usage: u16,
}
#[derive(Debug)]
pub struct Device {
    pub vid: u16,
    pub pid: u16,
    pub protocol: &'static str,
    pub leds: usize,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && (d.interface == 255 || i32::from(d.interface) == i.interface)
            && d.usage_page == i.usage_page
    });
    Identification {
        // Detector inventory is not a protocol implementation: every upstream
        // controller sends 264/65-byte feature reports with different layouts
        // (Surge RGB planes at 0x08/0x28/0x48, FPS/Dart/Haste/Raid direct frames,
        // Haste setup packet) that the audited branch version did not reproduce.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Mouse),
        reason: Some(if d.is_some() {
            "HyperX mouse detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no HyperX mouse HID detector for this interface".into()
        }),
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Device> {
    let d = DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && (d.interface == 255 || i32::from(d.interface) == i.interface)
            && d.usage_page == i.usage_page
    })?;
    crate::devices_table::DEVICES
        .iter()
        .find(|dev| dev.pid == d.pid)
}
