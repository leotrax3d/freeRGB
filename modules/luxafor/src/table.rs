// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x04D8;
pub const PID: u16 = 0xF372;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID;
    Identification {
        supported,
        model: supported.then(|| "Luxafor Flag".into()),
        kind: supported.then_some(DeviceKind::Accessory),
        reason: (!supported).then(|| "no Luxafor Flag detector for this VID/PID".into()),
    }
}
