// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
}
#[derive(Debug)]
pub struct Device {
    pub pid: u16,
    pub channels: usize,
    pub leds_per_channel: usize,
    pub version: u8,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid);
    Identification {
        // Detector inventory is not a protocol implementation: the audit found
        // byte-level defects in every version's Direct-mode path (V4 first-packet
        // capacity 316/340 and LowCount scheme, V1/V3 commit markers 0xA3/0xC8 and
        // div_ceil(21) packet accounting plus header shape, V2 0xBB marker being
        // overwritten by the payload) and the 500 ms keepalive that sustains colors
        // is not ported.
        supported: false,
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::LightingController),
        reason: Some(if d.is_some() {
            "DeepRGB detector is inventoried but its controller-specific protocol is not ported"
                .into()
        } else {
            "no DeepRGB HID detector for this VID/PID".into()
        }),
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Device> {
    crate::devices_table::DEVICES
        .iter()
        .find(|d| d.pid == i.pid)
}
