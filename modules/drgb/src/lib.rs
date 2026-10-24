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
pub const MODULE_ID: &str = "drgb";
#[derive(Default)]
pub struct Drgb {
    active: bool,
    pid: u16,
    channels: usize,
    leds_per_channel: usize,
    version: u8,
    leds: usize,
    zone_sizes: Vec<usize>,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Drgb {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "DRGBController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize_with_zones(
        &mut self,
        i: &InterfaceInfo,
        z: Option<&[Zone]>,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::device(i).ok_or_else(|| err("unsupported DRGB device"))?;
        self.pid = d.pid;
        self.channels = d.channels;
        self.leds_per_channel = d.leds_per_channel;
        self.version = d.version;
        self.zone_sizes = z
            .map(|x| x.iter().map(|q| q.leds.clamp(1, 8192)).collect())
            .unwrap_or_else(|| vec![d.leds_per_channel; d.channels]);
        self.channels = self.zone_sizes.len().clamp(1, 36);
        self.leds = self.zone_sizes.iter().sum::<usize>().clamp(1, 8192);
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i).model.unwrap_or_else(|| "DRGB".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: self.leds as u16,
                height: 1,
                leds: (0..self.leds)
                    .map(|n| Led {
                        name: format!("LED {}", n + 1),
                        pos: (n as u16, 0),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: self
                .zone_sizes
                .iter()
                .enumerate()
                .map(|(n, leds)| Zone {
                    name: format!("Channel {}", n + 1),
                    leds: *leds,
                })
                .collect(),
        })
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, h)
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.leds {
            return Err(err("invalid DRGB frame"));
        }
        let packets = match self.version {
            4 => report::v4_zoned(c, &self.zone_sizes),
            3 => report::v3_zoned(c, &self.zone_sizes, 0x60),
            1 => report::v3_zoned(c, &self.zone_sizes, 0x46),
            2 => report::v2_zoned(c, &self.zone_sizes),
            _ => return Err(err("unknown DRGB protocol version")),
        };
        for p in packets {
            h.write(&p).map_err(err)?;
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
