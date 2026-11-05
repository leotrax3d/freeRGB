// SPDX-License-Identifier: GPL-2.0-or-later
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, MODULE_API,
};
pub const MODULE_ID: &str = "lego-dimensions-toypad";
#[derive(Default)]
pub struct LegoDimensionsToypad {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for LegoDimensionsToypad {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "LegoDimensionsToypadBaseController".into(),
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
        h.write(&report::activate()).map_err(err)?;
        self.active = true;
        let leds = (0..3usize)
            .map(|n| Led {
                name: format!("LED {}", n + 1),
                pos: (n as u16, 0),
            })
            .collect();
        Ok(Capabilities {
            name: id
                .model
                .unwrap_or_else(|| "Lego Dimensions Toypad Base".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: 3,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 3 {
            return Err(err("invalid frame"));
        }
        for (zone, color) in colors.iter().enumerate() {
            let p = report::direct(zone, *color);
            h.write(&p).map_err(err)?;
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
