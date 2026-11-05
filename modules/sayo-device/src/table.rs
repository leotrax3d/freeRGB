// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x8089;
pub const PID: u16 = 0x0007;
pub const USAGE_PAGE: u16 = 0xFF11;
pub const USAGE: u16 = 0x0002;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "SayoDevice E1".into()),
        kind: supported.then_some(DeviceKind::LightingController),
        reason: (!supported).then(|| "no SayoDevice detector for this E1".into()),
    }
}
