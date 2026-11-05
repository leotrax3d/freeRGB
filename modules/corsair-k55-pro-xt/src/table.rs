// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub const VID: u16 = 0x1B1C;
pub const PID: u16 = 0x1BA1;
pub const USAGE_PAGE: u16 = 0xFF42;
pub const USAGE: u16 = 0x0000; // detector IPU only, no usage; match any usage
                               // The upstream REGISTER_HID_DETECTOR_IP requires usage page 0xFF42 but no specific usage.
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE;
    Identification {
        supported,
        model: supported.then(|| "Corsair K55 RGB PRO XT".into()),
        kind: supported.then_some(DeviceKind::Keyboard),
        reason: (!supported)
            .then(|| "no Corsair K55 RGB PRO XT detector for this interface".into()),
    }
}
