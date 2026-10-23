// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};
pub const MODULE_ID: &str = "nzxt-hue2";
#[derive(Default)]
pub struct Nzxt {
    channels: Vec<Vec<usize>>,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Nzxt {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "NZXTHue2Controller".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::device(i).ok_or_else(|| err("unsupported NZXT device"))?;
        // upstream: NZXTHue2Controller.cpp ReadDeviceList: request 20 03 and read
        // 64-byte replies until the device list answers (21 03).
        h.write(&report::device_list_request()).map_err(err)?;
        let mut reply = Vec::new();
        for _ in 0..10 {
            reply = h.read(64, 1000).map_err(err)?;
            if reply.len() == 64 && reply.starts_with(&[0x21, 3]) {
                break;
            }
        }
        if !(reply.len() == 64 && reply.starts_with(&[0x21, 3])) {
            return Err(err("NZXT device-list reply was not received"));
        }
        self.channels = report::parse_device_list(&reply, d.channels);
        let total: usize = self.channels.iter().map(|z| z.iter().sum::<usize>()).sum();
        if total == 0 {
            return Err(err("NZXT device-list reply contained no RGB devices"));
        }
        let mut leds = Vec::new();
        let mut zones = Vec::new();
        for (c, z) in self.channels.iter().enumerate() {
            let n = z.iter().sum::<usize>();
            zones.push(Zone {
                name: format!("Channel {}", c + 1),
                leds: n,
            });
            for k in 0..n {
                leds.push(Led {
                    name: format!("Channel {} LED {}", c + 1, k + 1),
                    pos: (k as u16, c as u16),
                });
            }
        }
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i).model.unwrap_or_else(|| "NZXT".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: leds.len().min(u16::MAX as usize) as u16,
                height: d.channels as u16,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones,
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active {
            return Err(err("NZXT module is not initialized"));
        }
        let total: usize = self.channels.iter().map(|z| z.iter().sum::<usize>()).sum();
        if colors.len() != total {
            return Err(err("invalid NZXT frame"));
        }
        // upstream: RGBController_NZXTHue2 DeviceUpdateLEDs calls
        // SetChannelLEDs for every zone: first direct group (up to 20), optional
        // second group, then apply — even for channels without devices.
        let mut off = 0;
        for (c, z) in self.channels.iter().enumerate() {
            let n = z.iter().sum::<usize>().min(2 * report::LED_PER_GROUP);
            h.write(&report::direct(
                c,
                0,
                &colors[off..off + n.min(report::LED_PER_GROUP)],
            ))
            .map_err(err)?;
            if n > report::LED_PER_GROUP {
                h.write(&report::direct(
                    c,
                    1,
                    &colors[off + report::LED_PER_GROUP..off + n],
                ))
                .map_err(err)?;
            }
            h.write(&report::apply(c)).map_err(err)?;
            off += self.channels[c].iter().sum::<usize>();
        }
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
