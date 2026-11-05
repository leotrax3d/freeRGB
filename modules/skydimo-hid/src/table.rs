// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x1A86;
pub const PID: u16 = 0xE316;
pub const USAGE_PAGE: u16 = 0xFF00;
pub const USAGE: u16 = 0x0001;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "Skydimo SK0902".into()),
        kind: supported.then_some(DeviceKind::LightingController),
        reason: (!supported).then(|| "no Skydimo HID detector for this interface".into()),
    }
}
