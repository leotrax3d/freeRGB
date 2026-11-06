// SPDX-License-Identifier: GPL-2.0-or-later
//! EVGA USB inventory module.
//!
//! The seven upstream detectors (Z15/Z20 keyboards, X20 mouse + receiver, X12
//! mouse) are inventoried, but the audited branch version fabricated every
//! Direct-mode packet; upstream protocols need the per-key packet_map,
//! feature-report queries and the X12 state machine. Nothing is claimed as
//! supported.
pub mod devices_table;
pub mod table;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, MODULE_API,
};
pub const MODULE_ID: &str = "evga-usb";
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
#[derive(Default)]
pub struct Evga;
impl freergb_module_api::sdk::Module for Evga {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "EVGAUSBController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize_with_zones(
        &mut self,
        _: &InterfaceInfo,
        _: Option<&[freergb_module_api::Zone]>,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        Err(err(
            "the EVGA protocol path is not ported; this device is unsupported",
        ))
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, h)
    }
    fn frame(&mut self, _: &[freergb_core::Rgb], _: &mut dyn Hid) -> Result<(), ModuleError> {
        Err(err("EVGA module is not initialized"))
    }
    fn shutdown(
        &mut self,
        _: Option<freergb_core::Rgb>,
        _: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}
#[cfg(test)]
mod tests;
