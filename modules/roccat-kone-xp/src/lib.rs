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
pub const MODULE_ID: &str = "roccat-kone-xp";

#[derive(Default)]
pub struct K {
    active: bool,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

impl Module for K {
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
        table::device(i).ok_or_else(|| err("unsupported Roccat Kone XP"))?;
        h.send_feature(&report::enter_direct()).map_err(err)?;
        self.active = true;
        let mut leds = Vec::new();
        for i in 0..9 {
            leds.push(Led { name: format!("Left LED {}", i + 1), pos: (i as u16, 0) });
        }
        for i in 0..9 {
            leds.push(Led { name: format!("Right LED {}", i + 1), pos: ((9 + i) as u16, 0) });
        }
        leds.push(Led { name: "Scrollwheel LED".into(), pos: (18, 0) });
        leds.push(Led { name: "DPI button LED".into(), pos: (19, 0) });
        Ok(Capabilities {
            name: table::identify(i).model.unwrap_or_else(|| "Roccat Kone XP".into()),
            kind: DeviceKind::Mouse,
            layout: LedLayout { width: 20, height: 1, leds },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![
                Zone { name: "Left".into(), leds: 9 },
                Zone { name: "Right".into(), leds: 9 },
                Zone { name: "Scrollwheel".into(), leds: 1 },
                Zone { name: "DPI button".into(), leds: 1 },
            ],
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != 20 {
            return Err(err("invalid Roccat Kone XP frame"));
        }
        h.send_feature(&report::direct(c)).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
