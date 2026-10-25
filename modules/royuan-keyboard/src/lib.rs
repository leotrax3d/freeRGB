// SPDX-License-Identifier: GPL-2.0-or-later
//! Royuan keyboard inventory module.
//!
//! The five upstream detectors are inventoried (Akko 3068B Plus, Skyloong GK68HE
//! Pro, Epomaker TH80 Pro USB/Dongle, Attack Shark K86); the audited branch
//! version used output writes with wrong packet layouts, while every upstream
//! path sends feature reports with per-protocol checksums. Nothing is claimed as
//! supported.
pub mod devices_table;
pub mod table;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, MODULE_API,
};
pub const MODULE_ID: &str = "royuan-keyboard";
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
#[derive(Default)]
pub struct Royuan;
impl freergb_module_api::sdk::Module for Royuan {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "RoyuanKeyboardController".into(),
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
            "the Royuan protocol path is not ported; this device is unsupported",
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
        Err(err("Royuan module is not initialized"))
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
