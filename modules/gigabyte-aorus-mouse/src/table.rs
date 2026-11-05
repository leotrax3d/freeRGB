// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x1044;
pub const PID: u16 = 0x7A40;
pub const USAGE_PAGE: u16 = 0xFF01;
pub const USAGE: u16 = 0x0001;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID
        && i.pid == PID
        && i.usage_page == USAGE_PAGE
        && i.usage == USAGE
        && i.interface == 3;
    Identification {
        supported,
        model: supported.then(|| "Gigabyte Aorus Mouse".into()),
        kind: supported.then_some(DeviceKind::Mouse),
        reason: (!supported).then(|| "no Gigabyte Aorus mouse detector for this interface".into()),
    }
}
