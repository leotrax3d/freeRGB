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
pub const MODULE_ID: &str = "corsair-peripheral-v2";
#[derive(Default)]
pub struct CorsairPeripheralV2 {
    active: bool,
    leds: usize,
    handler: &'static str,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for CorsairPeripheralV2 {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CorsairPeripheralV2Controller".into(),
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
        let handler = table::handler(i).ok_or_else(|| err("unsupported Corsair V2 HID device"))?;
        let leds = if handler.contains("Hardware") { 16 } else { 20 };
        let write_cmd = if handler.contains("Wireless") || handler.contains("Receiver") {
            9
        } else {
            8
        };
        h.write(&report::set_mode(write_cmd, 2)).map_err(err)?;
        h.write(&report::lighting_control(write_cmd)).map_err(err)?;
        self.leds = z
            .map(|x| x.iter().map(|q| q.leds).sum())
            .unwrap_or(leds)
            .clamp(1, 240);
        self.handler = handler;
        self.active = true;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Corsair V2".into()),
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
                name: "Corsair V2".into(),
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
            return Err(err("invalid Corsair V2 frame"));
        }
        let mut data = Vec::with_capacity(c.len() * 3);
        if self.handler.contains("Hardware") {
            data.extend_from_slice(&[2, 0]);
        }
        for color in c {
            data.extend_from_slice(&[color.r, color.g, color.b]);
        }
        let write_cmd = if self.handler.contains("Wireless") || self.handler.contains("Receiver") {
            9
        } else {
            8
        };
        for packet in report::block_packets(write_cmd, &data) {
            h.write(&packet).map_err(err)?;
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
