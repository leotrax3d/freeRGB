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
pub const MODULE_ID: &str = "roccat-burst-pro-air";
#[derive(Default)]
pub struct RoccatBurstProAir {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for RoccatBurstProAir {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "RoccatBurstProAirController".into(),
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
        let names = ["Scroll Wheel", "Logo", "Left button", "Right button"];
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Roccat Burst Pro Air".into()),
            kind: DeviceKind::Mouse,
            layout: LedLayout {
                width: 4,
                height: 1,
                leds: names
                    .iter()
                    .enumerate()
                    .map(|(idx, name)| Led {
                        name: (*name).into(),
                        pos: (idx as u16, 0),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 4 {
            return Err(err("invalid Roccat Burst Pro Air frame"));
        }
        let mut arr = [Rgb::default(); 4];
        arr.copy_from_slice(colors);
        h.send_feature(&report::direct(&arr)).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
