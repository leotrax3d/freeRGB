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
pub const MODULE_ID: &str = "coolermaster";
#[derive(Default)]
pub struct CoolerMaster {
    active: bool,
    leds: usize,
    handler: &'static str,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for CoolerMaster {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CoolerMasterController".into(),
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
        let id = table::identify(i);
        let handler =
            table::handler(i).ok_or_else(|| err("unsupported Cooler Master HID device"))?;
        let leds = if handler.contains("Keyboard") {
            87
        } else if handler.contains("Mouse") {
            2
        } else if handler.contains("ARGB") {
            24
        } else {
            20
        };
        if handler.contains("Keyboard") || handler.contains("ARGB") {
            h.write(&report::keyboard(&[])).map_err(err)?;
        }
        self.leds = z
            .map(|x| x.iter().map(|q| q.leds).sum())
            .unwrap_or(leds)
            .clamp(1, 240);
        self.handler = handler;
        self.active = true;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Cooler Master".into()),
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
            zones: vec![Zone {
                name: "Cooler Master".into(),
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
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.leds {
            return Err(err("invalid Cooler Master frame"));
        }
        let p = if self.handler.contains("Mouse") {
            report::mouse(c)
        } else if self.handler.contains("Keyboard") || self.handler.contains("ARGB") {
            report::keyboard(c)
        } else {
            report::direct(c)
        };
        h.write(&p).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
