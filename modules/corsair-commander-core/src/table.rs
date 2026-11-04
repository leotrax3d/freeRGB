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
}
pub fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = detector(i);
    if let Some(d) = d {
        return Identification {
            // upstream: CorsairCommanderCoreController.cpp routes all rows to
            // the full command protocol; the variant (V1/V2/V3/XT) is decided
            // by PID and the 0x02 0x13 version query in initialize(). The
            // Initialize -> SetFanMode -> GetLedCounts -> SetDirectColor
            // lifecycle is fully ported.
            supported: true,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::LightingController),
            reason: None,
        };
    }
    Identification {
        supported: false,
        model: None,
        kind: None,
        reason: Some("no Corsair Commander Core HID detector for this VID/PID".into()),
    }
}
