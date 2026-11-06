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
pub const MODULE_ID: &str = "roccat-horde-aimo";

#[derive(Default)]
pub struct H {
    active: bool,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

impl Module for H {
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
        table::device(i).ok_or_else(|| err("unsupported Roccat Horde Aimo"))?;
        // upstream: RoccatHordeAimoController.cpp constructor InitialPacket()
        h.send_feature(&report::init()).map_err(err)?;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Roccat Horde Aimo".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: report::LED_COUNT as u16,
                height: 1,
                leds: (0..report::LED_COUNT)
                    .map(|n| Led {
                        name: format!("LED {}", n + 1),
                        pos: (n as u16, 0),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![freergb_module_api::Zone {
                name: "Keyboard".into(),
                leds: report::LED_COUNT,
            }],
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != report::LED_COUNT {
            return Err(err("invalid Roccat Horde Aimo frame"));
        }
        // upstream: RGBController_RoccatHordeAimo.cpp DeviceUpdateLEDs() ->
        // SetColors: 24-byte feature write, then the 3-byte feature read
        h.send_feature(&report::set_colors(c)).map_err(err)?;
        let r = [0u8; 3];
        h.get_feature(&r, 3).map_err(err)?;

        Ok(())
    }

    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
