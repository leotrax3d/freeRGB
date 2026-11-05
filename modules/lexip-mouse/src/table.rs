// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x04D8;
pub const PID: u16 = 0xFD0A;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported =
        i.vid == VID && i.pid == PID && i.interface == 0 && i.usage_page == 2 && i.usage == 0x0001;
    Identification {
        supported,
        model: supported.then(|| "Np93 ALPHA - Gaming Mouse".into()),
        kind: supported.then_some(DeviceKind::Mouse),
        reason: (!supported).then(|| "no Lexip mouse detector for this interface".into()),
    }
}
