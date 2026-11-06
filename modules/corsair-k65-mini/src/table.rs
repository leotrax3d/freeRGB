// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x1B1C;
pub const PID: u16 = 0x1BAF;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.interface == 1;
    Identification {
        supported,
        model: supported.then(|| "Corsair K65 Mini".into()),
        kind: supported.then_some(DeviceKind::Keyboard),
        reason: (!supported).then(|| "no Corsair K65 Mini detector for this interface".into()),
    }
}
