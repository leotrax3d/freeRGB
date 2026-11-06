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
pub const MODULE_ID: &str = "roccat-sense-aimo";

#[derive(Default)]
pub struct S {
    active: bool,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

impl Module for S {
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
        _h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        table::device(i).ok_or_else(|| err("unsupported Roccat Sense Aimo"))?;
        // upstream: RoccatSenseAimoController ctor stores fields only; no
        // handshake. Effect modes are omitted by the module API (Direct only).
        self.active = true;
        let leds = vec![
            Led {
                name: "Mousepad left led".into(),
                pos: (0, 0),
            },
            Led {
                name: "Mousepad right led".into(),
                pos: (1, 0),
            },
        ];
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Roccat Sense Aimo".into()),
            kind: DeviceKind::Mousemat,
            layout: LedLayout {
                width: 2,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![freergb_module_api::Zone {
                name: "Mousepad".into(),
                leds: 2,
            }],
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != 2 {
            return Err(err("invalid Roccat Sense Aimo frame"));
        }
        // upstream: RGBController_RoccatSenseAimo.cpp DeviceUpdateLEDs() ->
        // SendDirect(colors) writes all two LEDs each frame
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
