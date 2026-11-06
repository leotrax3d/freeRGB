// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: i32,
    pub usage_page: u16,
    pub usage: u16,
}
pub const VID: u16 = 0x048D;
pub const PID: u16 = 0x7001;
pub const USAGE_PAGE: u16 = 0xFF03;
pub const USAGE: u16 = 0x02;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "CLEVO Lightbar".into()),
        kind: supported.then_some(DeviceKind::LightingController),
        reason: (!supported).then(|| "no Clevo Lightbar detector for this interface".into()),
    }
}
