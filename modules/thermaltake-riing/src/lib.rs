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
pub const MODULE_ID: &str = "thermaltake-riing";
#[derive(Default)]
pub struct Thermaltake {
    active: bool,
    zones: Vec<usize>,
    handler: &'static str,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Thermaltake {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "ThermaltakeRiingController".into(),
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
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let handler =
            table::handler(i).ok_or_else(|| err("unsupported Thermaltake Riing device"))?;
        h.write(&report::init::<65>()).map_err(err)?;
        let count = if handler.contains("Quad") {
            60
        } else if handler.contains("Trio") {
            54
        } else {
            20
        };
        self.zones = (0..5)
            .map(|n| {
                z.and_then(|x| x.get(n))
                    .map(|q| q.leds)
                    .unwrap_or(1)
                    .clamp(1, count)
            })
            .collect();
        self.handler = handler;
        self.active = true;
        let total = self.zones.iter().sum();
        let leds = (0..total)
            .map(|n| Led {
                name: format!("LED {}", n + 1),
                pos: (n as u16, 0),
            })
            .collect();
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Thermaltake Riing".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: total as u16,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: self
                .zones
                .iter()
                .enumerate()
                .map(|(n, &leds)| Zone {
                    name: format!("Thermaltake Fan Header {}", n + 1),
                    leds,
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
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != self.zones.iter().sum::<usize>() {
            return Err(err("invalid Thermaltake Riing frame"));
        }
        let mut offset = 0;
        for (zone, &len) in self.zones.iter().enumerate() {
            let slice = &colors[offset..offset + len];
            offset += len;
            if self.handler.contains("Quad") {
                h.write(&report::quad(zone, slice)).map_err(err)?;
            } else if self.handler.contains("Trio") {
                let chunks = len.max(1).div_ceil(19);
                for chunk in 0..chunks.max(2) {
                    let begin = chunk * 19;
                    h.write(&report::trio(
                        zone,
                        chunk,
                        &slice[begin..slice.len().min(begin + 19)],
                    ))
                    .map_err(err)?;
                }
            } else {
                h.write(&report::base(zone, slice)).map_err(err)?;
            }
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
