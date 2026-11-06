// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x0E6F;
pub const PID: u16 = 0x0241;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID;
    Identification {
        supported,
        model: supported.then(|| "Lego Dimensions Toypad Base".into()),
        kind: supported.then_some(DeviceKind::LightingController),
        reason: (!supported).then(|| "no Lego Dimensions toypad detector for this VID/PID".into()),
    }
}
