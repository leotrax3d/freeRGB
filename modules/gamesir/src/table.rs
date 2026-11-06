// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x3537;
pub const PID_1: u16 = 0x100F;
pub const PID_2: u16 = 0x1098;
pub const USAGE_PAGE: u16 = 0xFF7A;
pub const USAGE: u16 = 0x0001;
pub const INTERFACE: i32 = 2;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = (i.vid == VID)
        && (i.pid == PID_1 || i.pid == PID_2)
        && i.interface == INTERFACE
        && i.usage_page == USAGE_PAGE
        && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "GameSir RGB Device".into()),
        kind: supported.then_some(DeviceKind::Accessory),
        reason: (!supported).then(|| "no GameSir detector for this interface".into()),
    }
}
