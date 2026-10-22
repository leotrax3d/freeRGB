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
impl Detector {
    pub fn matches(&self, i: &InterfaceInfo) -> bool {
        self.vid == i.vid
            && self.pid == i.pid
            && self.interface == i.interface
            && self.usage_page == i.usage_page
    }
}

pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.matches(i));
    Identification {
        supported: d.is_some(),
        model: d.map(|d| d.name.to_owned()),
        kind: d.map(|_| DeviceKind::Mouse),
        reason: d
            .is_none()
            .then(|| "no Redragon detector for this interface".into()),
    }
}
