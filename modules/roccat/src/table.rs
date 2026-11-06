// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// 255 = any interface; 65535 = any usage page (upstream semantics).
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
        // Detector inventory is not a protocol implementation: the 28 upstream
        // rows dispatch to ~10 controllers (Burst/BurstProAir/Elo/Kone*/Kova/
        // Horde/Vulcan keyboards with key tables, Sense mice, Aimox) — each with
        // its own control-mode feature reports, direct packets and layouts. The
        // audited branch version had duplicated rows (56) and used output writes
        // where upstream sends feature reports.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "Roccat detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no Roccat HID detector for this interface".into()
        }),
    }
}
