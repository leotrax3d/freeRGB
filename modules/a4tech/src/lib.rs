// SPDX-License-Identifier: GPL-2.0-or-later
//! A4Tech (Bloody) inventory module.
//!
//! All six upstream detectors are inventoried, but the audited branch version was
//! not byte-exact (output writes instead of feature reports, missing Gain/Release
//! control packets, missing mouse per-LED index encoding). Nothing is claimed as
//! supported.
pub mod devices_table;
pub mod table;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, MODULE_API,
};
pub const MODULE_ID: &str = "a4tech";
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
#[derive(Default)]
pub struct A4Tech;
impl freergb_module_api::sdk::Module for A4Tech {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "A4TechController".into(),
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
            "the A4Tech protocol path is not ported; this device is unsupported",
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
        Err(err("A4Tech module is not initialized"))
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
