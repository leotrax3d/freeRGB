// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, MODULE_API,
};
pub const MODULE_ID: &str = "roccat";
#[derive(Default)]
pub struct Roccat {
    protocol: &'static str,
    leds: usize,
    packet: usize,
    column: usize,
    version: u8,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Roccat {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "RoccatController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::device(i).ok_or_else(|| err("unsupported Roccat device"))?;
        self.protocol = d.protocol;
        self.leds = d.leds;
        self.packet = d.packet;
        self.column = d.column;
        self.version = d.version;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i).model.unwrap_or_else(|| "Roccat".into()),
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
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != self.leds {
            return Err(err("invalid Roccat frame"));
        }
        match self.protocol {
            "keyboard" => {
                for p in report::keyboard(colors, self.packet, self.column, self.version, false) {
                    h.write(&p).map_err(err)?;
                }
            }
            "mat" => {
                h.write(&report::mat(colors)).map_err(err)?;
            }
            "headset" => {
                h.write(&report::headset(colors[0])).map_err(err)?;
            }
            "mouse" => {
                h.write(&report::mouse(colors, self.packet)).map_err(err)?;
            }
            "kova" => {
                h.write(&report::direct_mouse(colors, 0x0d, 0x2e, self.packet))
                    .map_err(err)?;
            }
            _ => {
                h.write(&report::direct_mouse(colors, 0x0d, 0x7a, self.packet))
                    .map_err(err)?;
            }
        };
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
