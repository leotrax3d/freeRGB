// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x1E71;
pub const PID: u16 = 0x2100;
pub const USAGE_PAGE: u16 = 0xFFCA;
pub const USAGE: u16 = 0x0001;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "NZXT Mouse".into()),
        kind: supported.then_some(DeviceKind::Mouse),
        reason: (!supported).then(|| "no NZXT mouse detector for this interface".into()),
    }
}
