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

pub const MODULE_ID: &str = "msi-motherboard";

#[derive(Default)]
pub struct MsiMotherboard {
    variant: usize,
    active: bool,
    zones: Vec<usize>,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

impl Module for MsiMotherboard {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "MSIMotherboardController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize_with_zones(
        &mut self,
        i: &InterfaceInfo,
        configured: Option<&[Zone]>,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(err(id.reason.unwrap_or_else(|| "unsupported".into())));
        }
        let probe = hid.get_feature(&[0x52], report::REPORT_185).map_err(err)?;
        self.variant = match probe.len() {
            112..=113 => 112,
            162..=163 => 162,
            185..=186 => 185,
            _ => 761,
        };
        let defaults = match self.variant {
            112 => vec![1, 1, 1, 7],
            162 => vec![1, 1, 1, 1, 10, 1],
            185 => vec![1, 1, 1, 30, 30, 30, 1],
            _ => vec![30, 30, 30, 30],
        };
        self.zones = configured
            .map(|z| z.iter().map(|x| x.leds).collect())
            .unwrap_or(defaults);
        if self.variant == 761 {
            hid.get_feature(&[0x50], report::REPORT_CONFIG)
                .map_err(err)?;
            hid.send_feature(&report::config_761(
                &self
                    .zones
                    .iter()
                    .map(|x| (*x).min(255) as u8)
                    .collect::<Vec<_>>(),
            ))
            .map_err(err)?;
        }
        self.active = true;
        let names = match self.variant {
            112 => vec!["JRGB1", "JRAINBOW1", "JCORSAIR", "Onboard LEDs"],
            162 => vec![
                "JRGB1",
                "JRAINBOW1",
                "JCORSAIR",
                "JCORSAIR outer",
                "Onboard LEDs",
                "JRGB2",
            ],
            185 => vec![
                "JRGB1",
                "JPIPE1",
                "JPIPE2",
                "JRAINBOW1",
                "JRAINBOW2",
                "JCORSAIR",
                "Onboard LEDs",
            ],
            _ => vec!["JAF", "JARGB1", "JARGB2", "JARGB3"],
        };
        let leds: Vec<Led> = self
            .zones
            .iter()
            .enumerate()
            .flat_map(|(z, n)| {
                let zone_name = names.get(z).copied().unwrap_or("Zone");
                (0..*n).map(move |l| Led {
                    name: format!("{} LED {}", zone_name, l + 1),
                    pos: (l as u16, z as u16),
                })
            })
            .collect();
        let zones = self
            .zones
            .iter()
            .enumerate()
            .map(|(n, leds)| Zone {
                name: names.get(n).copied().unwrap_or("Zone").into(),
                leds: *leds,
            })
            .collect();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "MSI Mystic Light".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: self.zones.iter().copied().max().unwrap_or(1) as u16,
                height: self.zones.len() as u16,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones,
        })
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, hid)
    }
    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active {
            return Err(err("MSI module is not initialized"));
        }
        if self.variant == 761 {
            let mut start = 0;
            for (idx, count) in self.zones.iter().enumerate() {
                let end = (start + *count).min(colors.len());
                let header = if idx == 0 { 0x08 } else { 0x04 };
                hid.send_feature(&report::per_led_761(
                    header,
                    idx as u8,
                    &colors[start..end],
                    *count,
                ))
                .map_err(err)?;
                start += *count;
            }
        } else {
            hid.send_feature(&report::zone_packet(self.variant, colors, &self.zones))
                .map_err(err)?;
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
