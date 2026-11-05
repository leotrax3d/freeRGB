// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x3938;
pub const PID: u16 = 0x1179;
pub const USAGE_PAGE: u16 = 0xFF19;
pub const USAGE: u16 = 0xFF19;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "AOC GM500".into()),
        kind: supported.then_some(DeviceKind::Mouse),
        reason: (!supported).then(|| "no AOC mouse detector for this interface".into()),
    }
}
