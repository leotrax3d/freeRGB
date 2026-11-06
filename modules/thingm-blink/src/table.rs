// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::{PID, USAGE, USAGE_PAGE, VID};
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
pub fn identify(i: &InterfaceInfo) -> Identification {
    let supported = i.vid == VID && i.pid == PID && i.usage_page == USAGE_PAGE && i.usage == USAGE;
    Identification {
        supported,
        model: supported.then(|| "ThingM blink(1) mk2".into()),
        kind: supported.then_some(DeviceKind::LightingController),
        reason: (!supported).then(|| "no ThingM Blink detector for this interface".into()),
    }
}
