// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x06A3;
pub const PID: u16 = 0x0DC5;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID;
    Identification {
        supported,
        model: supported.then(|| "MadCatz Cyborg Gaming Light".into()),
        kind: supported.then_some(DeviceKind::Accessory),
        reason: (!supported).then(|| "no MadCatz Cyborg detector for this VID/PID".into()),
    }
}
