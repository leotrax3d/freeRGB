// SPDX-License-Identifier: GPL-2.0-or-later
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, MODULE_API,
};
pub const MODULE_ID: &str = "ionico";
#[derive(Default)]
pub struct Ionico {
    active: bool,
    keyboard: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Ionico {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "IonicoController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }
        let keyboard = table::is_keyboard(i);
        if keyboard {
            h.send_feature(&report::keyboard_mode_direct())
                .map_err(err)?;
        } else {
            h.send_feature(&report::lightbar_mode_direct())
                .map_err(err)?;
        }
        self.active = true;
        self.keyboard = keyboard;
        let (count, _name_fmt) = if keyboard {
            (4usize, (0..4))
        } else {
            (22usize, (0..22))
        };
        let leds = (0..count)
            .map(|n| Led {
                name: if keyboard {
                    format!("Keyboard Zone {}", n + 1)
                } else {
                    format!("Bar Led {}", n + 1)
                },
                pos: (n as u16, 0),
            })
            .collect();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Ionico Device".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: count as u16,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active {
            return Err(err("not initialized"));
        }
        if self.keyboard {
            if colors.len() != 4 {
                return Err(err("invalid keyboard frame"));
            }
            for (i, c) in colors.iter().enumerate() {
                h.send_feature(&report::direct_keyboard(*c, i))
                    .map_err(err)?;
            }
        } else {
            if colors.len() != 22 {
                return Err(err("invalid lightbar frame"));
            }
            h.send_feature(&report::lightbar_init0()).map_err(err)?;
            h.write(&report::lightbar_write(colors)).map_err(err)?;
            h.send_feature(&report::lightbar_init1()).map_err(err)?;
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
