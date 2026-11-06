// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x2516;
pub const PID: u16 = 0x0169;
pub const USAGE_PAGE: u16 = 0xFF0A;
pub const USAGE: u16 = 0x0002;
pub const INTERFACE: i32 = 3;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID
        && i.pid == PID
        && i.interface == INTERFACE
        && i.usage_page == USAGE_PAGE
        && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "Cooler Master MM712".into()),
        kind: supported.then_some(DeviceKind::Mouse),
        reason: (!supported).then(|| "no CMM M712 detector for this interface".into()),
    }
}
