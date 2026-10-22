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
pub const MODULE_ID: &str = "logitech";
#[derive(Default)]
pub struct Logitech {
    active: bool,
    leds: usize,
    handler: &'static str,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Logitech {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "LogitechController".into(),
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
        let handler = table::handler(i).ok_or_else(|| err("unsupported Logitech HID device"))?;
        if !id.supported {
            return Err(err("unsupported Logitech HID device"));
        }
        let default_leds = if handler.contains("G933") {
            2
        } else if handler.contains("X56") {
            1
        } else if handler.contains("KeyboardG213") {
            5
        } else if handler.contains("MouseG203") {
            3
        } else if handler.contains("KeyboardG815") || handler.contains("KeyboardG915") {
            117
        } else {
            20
        };
        self.leds = z
            .map(|x| x.iter().map(|q| q.leds).sum())
            .unwrap_or(default_leds)
            .clamp(1, 240);
        self.active = true;
        self.handler = handler;
        if handler.contains("MouseG203L") {
            h.write(&report::g203l_init()).map_err(err)?;
        }
        if handler.contains("KeyboardG815") {
            for p in report::g815_init() {
                h.write(&p).map_err(err)?;
            }
        }
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Logitech".into()),
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
                name: "Logitech".into(),
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
            return Err(err("invalid Logitech frame"));
        }
        if self.handler.contains("G933") {
            for (zone, color) in c.iter().enumerate() {
                h.write(&report::g933(zone, *color)).map_err(err)?;
            }
        } else if self.handler.contains("X56") {
            h.send_feature(&report::x56_brightness(100)).map_err(err)?;
            h.send_feature(&report::x56_color(c[0])).map_err(err)?;
        } else if self.handler.contains("KeyboardG213") {
            for (zone, color) in c.iter().enumerate() {
                h.write(&report::g213(zone, *color)).map_err(err)?;
            }
        } else if self.handler.contains("MouseG203L") {
            h.write(&report::g203l(c)).map_err(err)?;
            h.write(&report::g203_apply()).map_err(err)?;
        } else if self.handler.contains("MouseG203") {
            h.write(&report::g203(c)).map_err(err)?;
            h.write(&report::g203_apply()).map_err(err)?;
        } else if self.handler.contains("MouseG303")
            || self.handler.contains("MouseG403")
            || self.handler.contains("MouseGPRO")
        {
            let function = if self.handler.contains("MouseGPRO") {
                0x3c
            } else {
                0x3a
            };
            for (zone, color) in c.iter().enumerate() {
                h.write(&report::lightsync(function, zone, *color))
                    .map_err(err)?;
            }
        } else if self.handler.contains("KeyboardG815") {
            for (n, color) in c.iter().enumerate() {
                h.write(&report::g815_little(0x04u8.saturating_add(n as u8), *color))
                    .map_err(err)?;
            }
        } else if self.handler.contains("KeyboardG915") {
            let wired = self.handler.contains("Wired");
            for (n, color) in c.iter().enumerate() {
                h.write(&report::g915_little(
                    wired,
                    0x04u8.saturating_add(n as u8),
                    *color,
                ))
                .map_err(err)?;
            }
        } else {
            for x in c.chunks(20) {
                h.write(&report::direct(x)).map_err(err)?;
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
