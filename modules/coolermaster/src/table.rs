// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    /// 65535 = any usage page, as in the upstream detector.
    pub usage_page: u16,
    pub usage: u16,
}
fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && (d.interface == 255 || i32::from(d.interface) == i.interface)
            && (d.usage_page == 65535 || d.usage_page == i.usage_page)
    })
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = detector(i);
    Identification {
        // Detector inventory is not a protocol implementation: the 38 upstream
        // rows dispatch to 11 different controllers (CM MM/ARGB/RGB/MP750/R6000/
        // Monitor/GD160/Keyboard variants) each with their own packet framing,
        // init/get-status reads and per-device layouts; the audited branch version
        // used three fabricated generic packets for all of them.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "Cooler Master detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no Cooler Master HID detector for this interface".into()
        }),
    }
}
