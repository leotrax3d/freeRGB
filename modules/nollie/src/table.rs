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
}
#[derive(Debug)]
pub struct Device {
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    pub channels: usize,
    pub leds: usize,
    /// "high" = 1025-byte SendPacket path, "fixed" = 65-byte SendPacketFS chunks
    /// followed by SendUpdate (upstream NollieController.cpp SetChannelLEDs).
    /// Only the "fixed" path is fully ported; the "high" path needs the per-device
    /// channel_index maps, sorted channel order and flag-channel writes that
    /// upstream performs in RGBController_Nollie.cpp DeviceUpdateLEDs.
    pub protocol: &'static str,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    match device_with_name(i) {
        Some((_d, name, protocol)) => {
            if protocol == "fixed" {
                Identification {
                    supported: true,
                    model: Some(name.into()),
                    kind: Some(DeviceKind::LightingController),
                    reason: None,
                }
            } else {
                Identification {
                    supported: false,
                    model: Some(name.into()),
                    kind: Some(DeviceKind::LightingController),
                    reason: Some(
                        "Nollie high-speed devices need the upstream channel_index maps, sorted channel order and flag-channel writes, which are not ported".into(),
                    ),
                }
            }
        }
        None => Identification {
            supported: false,
            model: None,
            kind: None,
            reason: Some("no Nollie HID detector for this interface".into()),
        },
    }
}
pub fn device_with_name(
    i: &InterfaceInfo,
) -> Option<(&'static Device, &'static str, &'static str)> {
    use crate::devices_table::DEVICES;
    let d = DETECTORS
        .iter()
        .find(|d| d.vid == i.vid && d.pid == i.pid && i32::from(d.interface) == i.interface)?;
    let dev = DEVICES
        .iter()
        .find(|dev| dev.vid == d.vid && dev.pid == d.pid && dev.interface == d.interface)?;
    Some((dev, d.name, dev.protocol))
}
