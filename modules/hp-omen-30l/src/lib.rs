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
pub const MODULE_ID: &str = "hp-omen-30l";
#[derive(Default)]
pub struct HpOmen30L {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for HpOmen30L {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "HPOmen30LController".into(),
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
        let names = [
            "Logo",
            "Bar",
            "Fan",
            "CPU",
            "Bottom Fan",
            "Mid Fan",
            "Top Fan",
        ];
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "HP Omen 30L".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: 7,
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
        if !self.active || colors.len() != 7 {
            return Err(err("invalid HP Omen 30L frame"));
        }
        for (idx, color) in colors.iter().enumerate() {
            h.write(&report::direct((idx as u8) + 1, *color))
                .map_err(err)?;
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
