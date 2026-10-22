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
pub const MODULE_ID: &str = "corsair-peripheral";
#[derive(Default)]
pub struct CorsairPeripheral {
    active: bool,
    leds: usize,
    handler: &'static str,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for CorsairPeripheral {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CorsairPeripheralController".into(),
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
        let handler = table::handler(i).ok_or_else(|| err("unsupported Corsair peripheral"))?;
        let k65 = handler.contains("K65Mini");
        let leds = if k65 {
            18
        } else if handler.contains("K55RGBPROXT") {
            117
        } else if handler.contains("Peripheral") {
            if i.pid == 0x0a34 {
                9
            } else if i.pid == 0x1b3b {
                15
            } else if i.pid < 0x1b30 {
                3
            } else {
                20
            }
        } else {
            20
        };
        if k65 {
            for p in report::k65_init() {
                h.write(&p).map_err(err)?;
            }
        } else if handler.contains("Peripheral") || handler.contains("K55RGBPROXT") {
            for p in report::init() {
                h.write(&p).map_err(err)?;
            }
        }
        self.leds = z
            .map(|x| x.iter().map(|q| q.leds).sum())
            .unwrap_or(leds)
            .clamp(1, 240);
        self.handler = handler;
        self.active = true;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Corsair".into()),
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
                name: "Corsair".into(),
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
            return Err(err("invalid Corsair frame"));
        }
        let p = if self.handler.contains("K65Mini") {
            report::k65(c)
        } else if self.handler.contains("K55RGBPROXT") {
            report::keyboard(c)
        } else if self.handler.contains("ST100") {
            report::strip(c)
        } else {
            report::mouse(c)
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
