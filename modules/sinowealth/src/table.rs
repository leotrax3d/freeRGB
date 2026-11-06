// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// 255 = any interface; 65535 = any usage page.
    pub interface: u8,
    pub usage_page: u16,
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
        // Detector inventory is not a protocol implementation: the family spans
        // mice (Glorious Model O/D wired + wireless with dongle/cable detection,
        // Everest GT-100), the ZET Fury Pro, Genesis Xenon 200, Genesis Thor 300
        // and the Sinowealth keyboards (010C/0016) — with different packet sizes
        // (59/154/520/1032), plane orders, feature/write transports and init
        // sequences. The audited branch version had duplicated rows (26) and
        // reused builders across controllers.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "Sinowealth detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no Sinowealth HID detector for this interface".into()
        }),
    }
}
