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
pub const MODULE_ID: &str = "gigabyte-aorus-laptop";
/// upstream: GIGABYTE_AORUS_LAPTOP_BRIGHTNESS_MAX/2 default
pub const BRIGHTNESS_DEFAULT: u8 = 0x19;
#[derive(Default)]
pub struct Aorus {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Aorus {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "GigabyteAorusLaptopController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize_with_zones(
        &mut self,
        i: &InterfaceInfo,
        _: Option<&[freergb_module_api::Zone]>,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(err(id.reason.unwrap_or_else(|| "unsupported".into())));
        }
        // upstream: RGBController_GigabyteAorusLaptop.cpp SetupZones exposes the
        // whole keyboard as a single LED.
        self.active = true;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Gigabyte Aorus Laptop".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 1,
                height: 1,
                leds: vec![Led {
                    name: "Keyboard".into(),
                    pos: (0, 0),
                }],
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, h)
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 1 {
            return Err(err("invalid Gigabyte Aorus frame"));
        }
        h.send_feature(&report::direct(colors[0], BRIGHTNESS_DEFAULT))
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
