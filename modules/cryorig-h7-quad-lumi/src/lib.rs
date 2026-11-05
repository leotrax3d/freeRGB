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
pub const MODULE_ID: &str = "cryorig-h7-quad-lumi";
#[derive(Default)]
pub struct CryorigH7QuadLumi {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for CryorigH7QuadLumi {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CryorigH7QuadLumiController".into(),
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
        let mut names = Vec::new();
        for zone in ["Logo", "Underglow"] {
            for led_idx in 1..=5 {
                names.push(format!("{}, LED {}", zone, led_idx));
            }
        }
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "CRYORIG H7 Quad Lumi".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: 10,
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
        if !self.active || colors.len() != 10 {
            return Err(err("invalid Cryorig H7 Quad Lumi frame"));
        }
        let mut ch0 = [Rgb::default(); 5];
        ch0.copy_from_slice(&colors[0..5]);
        let mut ch1 = [Rgb::default(); 5];
        ch1.copy_from_slice(&colors[5..10]);
        h.write(&report::direct(0, &ch0)).map_err(err)?;
        h.write(&report::direct(1, &ch1)).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
