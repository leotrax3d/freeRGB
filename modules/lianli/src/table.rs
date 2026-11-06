// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    pub usage_page: u16,
    pub usage: u16,
}
fn detector(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid && i32::from(d.interface) == i.interface)
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = detector(i);
    Identification {
        // Detector inventory is not a protocol implementation: each row dispatches
        // to its own UniHub/Strimer/GAII controller with unique framing. Note:
        // the upstream LianLiUniHubSL* sources are GPL-2.0-only, which is a
        // licensing constraint for those paths under this module's
        // GPL-2.0-or-later license.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::LightingController),
        reason: Some(if d.is_some() {
            "Lian Li detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no Lian Li HID detector for this interface".into()
        }),
    }
}
