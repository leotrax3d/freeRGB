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
    pub usage_page: u16,
    pub usage: u16,
    /// "b820" keyboard or "mouse" mice/mousemat protocol.
    pub protocol: &'static str,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && i32::from(d.interface) == i.interface
            && d.usage_page == i.usage_page
            && d.usage == i.usage
    });
    Identification {
        // Detector inventory is not a protocol implementation: the audited branch
        // version used output writes instead of the required 64-byte feature
        // reports, missed the Gain/Release control packets on both protocols and
        // the mouse per-LED physical index encoding (RGBController_BloodyMouse
        // DeviceUpdateLEDs packs the index into the RGBColor high byte).
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Mouse),
        reason: Some(if d.is_some() {
            "A4Tech detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no A4Tech HID detector for this interface".into()
        }),
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
