// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)] pub struct Detector { pub name: &'static str, pub handler: &'static str, pub vid: u16, pub pid: u16 }
pub fn identify(i: &InterfaceInfo) -> Identification { let d=DETECTORS.iter().find(|d|d.vid==i.vid&&d.pid==i.pid); Identification{supported:d.is_some(),model:d.map(|d|d.name.into()),kind:d.map(|_|DeviceKind::Accessory),reason:d.is_none().then(||"no Logitech HID detector for this VID/PID".into())} }
