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
pub const MODULE_ID: &str = "roccat-elo";

#[derive(Default)]
pub struct E {
    active: bool,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

impl Module for E {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "RoccatController".into(),
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
        table::device(i).ok_or_else(|| err("unsupported Roccat Elo device"))?;
        // upstream: RoccatEloController.cpp constructor calls SendInit()
        for p in report::init() {
            h.write(&p).map_err(err)?;
        }
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i).model.unwrap_or_else(|| "Roccat Elo 7.1".into()),
            kind: DeviceKind::Headset,
            layout: LedLayout {
                width: 1,
                height: 1,
                leds: vec![Led { name: "Headset LED".into(), pos: (0, 0) }],
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![freergb_module_api::Zone { name: "Headset".into(), leds: 1 }],
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != 1 {
            return Err(err("invalid Roccat Elo frame"));
        }
        // upstream: RGBController_RoccatElo.cpp DeviceUpdateLEDs()
        h.write(&report::direct(&c[0])).map_err(err)?;
        Ok(())
    }

    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
