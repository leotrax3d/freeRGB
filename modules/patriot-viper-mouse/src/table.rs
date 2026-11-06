// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x0C45;
pub const PID: u16 = 0x7E18;
pub const USAGE_PAGE: u16 = 0xFF18;
pub const USAGE: u16 = 0x01;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "Patriot Viper V550".into()),
        kind: supported.then_some(DeviceKind::Mouse),
        reason: (!supported).then(|| "no Patriot Viper detector for this interface".into()),
    }
}
