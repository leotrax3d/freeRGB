// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x195D;
pub const PID: u16 = 0x2061;
pub const INTERFACE: i32 = 2;
pub const USAGE_PAGE: u16 = 0xFFC2;
pub const USAGE: u16 = 0x04;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID
        && i.pid == PID
        && i.interface == INTERFACE
        && i.usage_page == USAGE_PAGE
        && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "Dark Project KD3B V2".into()),
        kind: supported.then_some(DeviceKind::Keyboard),
        reason: (!supported).then(|| "no Dark Project detector for this interface".into()),
    }
}
