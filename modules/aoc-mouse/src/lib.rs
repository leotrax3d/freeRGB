// SPDX-License-Identifier: GPL-2.0-or-later
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};
pub const MODULE_ID: &str = "aoc-mouse";
#[derive(Default)]
pub struct AocMouse {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for AocMouse {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "AOCMouseController".into(),
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
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }
        self.active = true;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "AOC GM500".into()),
            kind: DeviceKind::Mouse,
            layout: LedLayout {
                width: 2,
                height: 1,
                leds: vec![
                    Led {
                        name: "Logo".into(),
                        pos: (0, 0),
                    },
                    Led {
                        name: "Scroll Wheel".into(),
                        pos: (1, 0),
                    },
                ],
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![
                Zone {
                    name: "Logo".into(),
                    leds: 1,
                },
                Zone {
                    name: "Scroll Wheel".into(),
                    leds: 1,
                },
            ],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 2 {
            return Err(err("invalid frame"));
        }
        h.send_feature(&report::direct(colors[0], colors[1]))
            .map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
