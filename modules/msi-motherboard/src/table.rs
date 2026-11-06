// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};

#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: i32,
    pub usage_page: u16,
}

pub fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && d.interface == i.interface
            && d.usage_page == i.usage_page
    })
}

pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = detector(i);
    Identification {
        // upstream: MSIMotherboardControllerDetect.cpp routes every row to the
        // same runtime probe (DetectMSIMotherboardControllers); the variant is
        // decided by the 0x52 feature-report length. The 112-byte controller
        // path is fully ported; initialize() fails closed for boards that
        // answer with the 162/185/761 layouts.
        supported: d.is_some(),
        model: d.map(|d| d.name.to_owned()),
        kind: d.map(|_| DeviceKind::LightingController),
        reason: if d.is_some() {
            None
        } else {
            Some("no MSI Mystic Light HID detector for this interface".into())
        },
    }
}
