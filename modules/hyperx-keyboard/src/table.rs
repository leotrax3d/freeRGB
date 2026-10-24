// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// 255 = any interface, as in the upstream detector.
    pub interface: u8,
    /// 0xffff = any usage page, as in the upstream detector.
    pub usage_page: u16,
    pub kind: &'static str,
}
/// Protocols whose Direct-mode path is fully ported and byte-verified:
/// Alloy Elite RGB (channel packets + extended media/indicator packet) and
/// Alloy FPS RGB (channel packets with the upstream KEYS table). Both use
/// 264-byte feature reports upstream.
pub const SUPPORTED: &[&str] = &["DetectHyperXAlloyElite", "DetectHyperXAlloyFPS"];
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && (d.interface == 255 || i32::from(d.interface) == i.interface)
            && (d.usage_page == 0xffff || d.usage_page == i.usage_page)
    });
    match d {
        Some(d) if SUPPORTED.contains(&d.handler) => Identification {
            supported: true,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Keyboard),
            reason: None,
        },
        Some(d) => Identification {
            supported: false,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Keyboard),
            reason: Some(format!(
                "HyperX detector {} is inventoried but its {} protocol path is not fully ported (audit)",
                d.name, d.kind
            )),
        },
        None => Identification {
            supported: false,
            model: None,
            kind: None,
            reason: Some("no HyperX keyboard HID detector for this interface".into()),
        },
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && (d.interface == 255 || i32::from(d.interface) == i.interface)
            && (d.usage_page == 0xffff || d.usage_page == i.usage_page)
    })
}
