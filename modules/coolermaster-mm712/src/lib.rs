// SPDX-License-Identifier: GPL-2.0-or-later
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, MODULE_API,
};
pub const MODULE_ID: &str = "coolermaster-mm712";
#[derive(Default)]
pub struct CoolerMasterMm712 {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for CoolerMasterMm712 {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CMM712".into(),
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
        let _ = h.send_feature(&report::init_normal());
        let _ = h.read(65, 1000);
        let _ = h.send_feature(&report::mode_status());
        let _ = h.read(65, 1000);
        let _ = h.send_feature(&report::color_status(0x02));
        let _ = h.read(65, 1000);
        h.send_feature(&report::direct_init()).map_err(err)?;
        self.active = true;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Cooler Master MM712".into()),
            kind: DeviceKind::Mouse,
            layout: LedLayout {
                width: 1,
                height: 1,
                leds: vec![Led {
                    name: "Logo LED".into(),
                    pos: (0, 0),
                }],
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 1 {
            return Err(err("invalid frame"));
        }
        h.send_feature(&report::direct_color(colors[0]))
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
