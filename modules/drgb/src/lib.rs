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
    channel_counts: Vec<usize>,
    version: u8,
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
        zones: Option<&[Zone]>,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }
        let dev = crate::devices_table::DEVICES
            .iter()
            .find(|d| d.pid == i.pid)
            .ok_or_else(|| err("no DeepRGB detector"))?;
        if dev.version != 1 && dev.version != 2 && dev.version != 3 && dev.version != 4 {
            return Err(err("DeepRGB version is not ported yet"));
        }
        let channel_counts = match zones {
            Some(z) if !z.is_empty() => z.iter().map(|z| z.leds).collect(),
            _ => vec![dev.leds_per_channel; dev.channels],
        };
        let total: usize = channel_counts.iter().sum();
        if total > 8192 {
            return Err(err("DeepRGB frame exceeds 8192 LED Direct-mode limit"));
        }
        let leds = (0..total)
            .map(|n| Led {
                name: format!("LED {}", n),
                pos: ((n % 30) as u16, (n / 30) as u16),
            })
            .collect::<Vec<_>>();
        let zones_out = if zones.is_none_or(|z| z.is_empty()) {
            channel_counts
                .iter()
                .enumerate()
                .map(|(i, &c)| Zone {
                    name: format!("Channel {}", i + 1),
                    leds: c,
                })
                .collect()
        } else {
            zones.unwrap().to_vec()
        };
        self.channel_counts = channel_counts;
        self.version = dev.version;
        Ok(Capabilities {
            name: "DRGB".into(),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: total as u16,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: zones_out,
        })
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, h)
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if self.channel_counts.is_empty() {
            return Err(err("DeepRGB module is not initialized"));
        }
        let total: usize = self.channel_counts.iter().sum();
        if colors.len() != total {
            return Err(err("invalid DeepRGB frame length"));
        }
        for packet in report::frames(self.version, &self.channel_counts, colors) {
            h.write(&packet).map_err(err)?;
        }
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        Ok(())
    }
}
#[cfg(test)]
mod tests;
