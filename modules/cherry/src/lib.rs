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
pub const MODULE_ID: &str = "cherry";
#[derive(Default)]
pub struct Cherry {
    leds: usize,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Cherry {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CherryKeyboardController".into(),
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
        if table::identify(i).supported {
            // upstream: RGBController_CherryKeyboard.cpp: CHERRY_MATRIX_CELL_COUNT
            // = CHERRY_MATRIX_MAP_HEIGHT (6) * CHERRY_MATRIX_MAP_WIDTH (21) = 126 LEDs.
            self.leds = 126;
            self.active = true;
        } else {
            return Err(err("unsupported Cherry device"));
        }
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Cherry Keyboard".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 21,
                height: 6,
                leds: (0..self.leds)
                    .map(|n| Led {
                        name: format!("Keyboard LED {}", n),
                        pos: ((n % 21) as u16, (n / 21) as u16),
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
            return Err(err("invalid Cherry frame"));
        }
        for packet in report::colors(colors) {
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
