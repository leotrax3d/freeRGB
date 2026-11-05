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
pub const MODULE_ID: &str = "msi-3-zone";
#[derive(Default)]
pub struct Msi3Zone {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Msi3Zone {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "MSI3ZoneController".into(),
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
            name: id.model.unwrap_or_else(|| "MSI 3-Zone Laptop".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 4,
                height: 1,
                leds: vec![
                    Led { name: "Keyboard Left".into(), pos: (0, 0) },
                    Led { name: "Keyboard Middle".into(), pos: (1, 0) },
                    Led { name: "Keyboard Right".into(), pos: (2, 0) },
                    Led { name: "Aux".into(), pos: (3, 0) },
                ],
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![
                Zone { name: "Keyboard".into(), leds: 3 },
                Zone { name: "Aux".into(), leds: 1 },
            ],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 4 {
            return Err(err("invalid MSI 3-zone frame"));
        }
        for packet in report::frames(colors) {
            h.send_feature(&packet).map_err(err)?;
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