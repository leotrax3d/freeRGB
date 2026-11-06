// SPDX-License-Identifier: GPL-2.0-or-later
//! Fnatic Streak inventory module.
//!
//! The three upstream keyboard detectors are inventoried, but the audited branch
//! version sent a single fabricated feature report for a 1-LED layout that matches
//! neither the fragmented SendRequest protocol (full 124 / mini 106 LEDs) nor the
//! Streak65 direct packet. Nothing is claimed as supported.
pub mod devices_table;
pub mod table;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, MODULE_API,
};
pub const MODULE_ID: &str = "fnatic-streak";
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
#[derive(Default)]
pub struct A;
impl freergb_module_api::sdk::Module for A {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "FnaticStreakController".into(),
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
            "the Fnatic Streak protocol path is not ported; this device is unsupported",
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
        Err(err("Fnatic Streak module is not initialized"))
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
