// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x048D;
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = if i.vid == VID && i.pid == 0xCE00 && i.usage_page == 0xFF12 && i.usage == 0x01
    {
        true
    } else {
        i.vid == VID && i.pid == 0x6005 && i.usage_page == 0xFF03 && i.usage == 0x01
    };
    let is_keyboard = i.pid == 0xCE00;
    Identification {
        supported,
        model: supported.then(|| {
            if is_keyboard {
                "Ionico Keyboard".into()
            } else {
                "Ionico Light Bar".into()
            }
        }),
        kind: supported.then_some(DeviceKind::Keyboard),
        reason: (!supported).then(|| "no Ionico detector for this interface".into()),
    }
}
pub fn is_keyboard(i: &InterfaceInfo) -> bool {
    i.pid == 0xCE00
}
