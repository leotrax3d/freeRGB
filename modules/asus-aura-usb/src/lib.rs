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
pub const MODULE_ID: &str = "asus-aura-usb";
#[derive(Default)]
pub struct AsusAuraUsb {
    active: bool,
    leds: usize,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for AsusAuraUsb {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "AsusAuraUSBController".into(),
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
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(err(id.reason.unwrap_or_else(|| "unsupported".into())));
        }
        hid.write(&report::config_request()).map_err(err)?;
        let config = hid.read(65, 100).map_err(err)?;
        let default_leds = config.get(0x1b).copied().unwrap_or(1).max(1) as usize;
        self.leds = zones
            .map(|z| z.iter().map(|x| x.leds).sum())
            .unwrap_or(default_leds)
            .clamp(1, 240);
        self.active = true;
        let leds = (0..self.leds)
            .map(|n| Led {
                name: format!("Aura LED {}", n + 1),
                pos: (n as u16, 0),
            })
            .collect();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "ASUS Aura USB".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: self.leds as u16,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![Zone {
                name: "Aura".into(),
                leds: self.leds,
            }],
        })
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, h)
    }
    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != self.leds {
            return Err(err("invalid Aura frame"));
        }
        for (offset, chunk) in colors.chunks(report::LEDS_PER_PACKET).enumerate() {
            let last = offset + report::LEDS_PER_PACKET >= colors.len();
            hid.write(&report::direct(
                0,
                offset * report::LEDS_PER_PACKET,
                chunk,
                last,
            ))
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
