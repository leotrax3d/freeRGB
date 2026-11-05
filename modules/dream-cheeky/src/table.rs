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
pub const VID: u16 = 0x1D34;
pub const PID: u16 = 0x0004;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID;
    Identification {
        supported,
        model: supported.then(|| "Dream Cheeky Webmail Notifier".into()),
        kind: supported.then_some(DeviceKind::Accessory),
        reason: (!supported).then(|| "no Dream Cheeky detector for this VID/PID".into()),
    }
}
