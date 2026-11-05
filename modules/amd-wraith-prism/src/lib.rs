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
pub const MODULE_ID: &str = "amd-wraith-prism";
#[derive(Default)]
pub struct AMDWraithPrism {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for AMDWraithPrism {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "AMDWraithPrismController".into(),
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
        self.active = true;
        h.write(&report::enable_direct()).map_err(err)?;
        h.write(&report::apply()).map_err(err)?;
        let mut names = vec!["Logo LED".to_string(), "Fan LED".to_string()];
        for i in 1..=15 {
            names.push(format!("Ring LED {}", i));
        }
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "AMD Wraith Prism".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: 17,
                height: 1,
                leds: names
                    .iter()
                    .enumerate()
                    .map(|(idx, name)| Led {
                        name: name.clone(),
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
        if !self.active || colors.len() != 17 {
            return Err(err("invalid AMD Wraith Prism frame"));
        }
        h.write(&report::packet(&report::LOGO_FAN_IDS, &colors[0..2]))
            .map_err(err)?;
        h.write(&report::packet(&report::RING_IDS, &colors[2..17]))
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
