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
pub const VID: u16 = 0x04D9;
pub const PID: u16 = 0xFC05;
pub const USAGE_PAGE: u16 = 0xFFA0;
pub const USAGE: u16 = 0x01;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "Tecknet M008".into()),
        kind: supported.then_some(DeviceKind::Mouse),
        reason: (!supported).then(|| "no Tecknet detector for this interface".into()),
    }
}