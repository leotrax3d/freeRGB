// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// 255 = any interface; 65535 = any usage page (upstream detector semantics).
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
        // Detector inventory is not a protocol implementation: the upstream V2
        // protocol needs StartTransaction/SubmitData/EndTransaction framing with
        // the hardware/software controller split, per-key layouts and read
        // validation; the audited branch version sent fabricated packets.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Accessory),
        reason: Some(if d.is_some() {
            "Corsair V2 detector is inventoried but its transaction protocol is not ported".into()
        } else {
            "no Corsair V2 HID detector for this interface".into()
        }),
    }
}
