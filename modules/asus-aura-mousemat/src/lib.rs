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
pub const MODULE_ID: &str = "asus-aura-mousemat";

#[derive(Default)]
pub struct M {
    active: bool,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

impl Module for M {
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

    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        _h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        table::device(i).ok_or_else(|| err("unsupported ASUS Aura mousemat"))?;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i).model.unwrap_or_else(|| "ASUS Aura Mousemat".into()),
            kind: DeviceKind::Mousemat,
            layout: LedLayout {
                width: 15,
                height: 1,
                leds: (0..15)
                    .map(|n| Led { name: format!("Mousemat LED {}", n), pos: (n as u16, 0) })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![freergb_module_api::Zone { name: "Mousemat".into(), leds: 15 }],
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != 15 {
            return Err(err("invalid ASUS Aura mousemat frame"));
        }
        h.write(&report::direct(c)).map_err(err)?;
        Ok(())
    }

    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
