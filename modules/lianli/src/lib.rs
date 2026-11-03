// SPDX-License-Identifier: GPL-2.0-or-later
//! Lian Li inventory module.
//!
//! The nine USB-HID detector rows (Uni Hub SL/AL, SL/AL V2, SL V2 v0.5, SL
//! Infinity, Strimer L Connect, GA II Trinity, GA II Trinity Performance) are
//! inventoried with interface/usage-page qualifications. The generic
//! REGISTER_DETECTOR rows (Uni Hub legacy, Universal Screen, AL10) are non-HID
//! USB transports and blocked under freeRGB policy. Each HID row dispatches to
//! its own controller; nothing is claimed as supported.
//!
//! Licensing note: the upstream LianLiUniHubSL sources carry
//! GPL-2.0-only SPDX headers; porting those paths would constrain the module
//! license accordingly.
pub mod devices_table;
pub mod table;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, MODULE_API,
};
pub const MODULE_ID: &str = "lianli";
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
#[derive(Default)]
pub struct LianLi;
impl freergb_module_api::sdk::Module for LianLi {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "LianLiController".into(),
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
            "the Lian Li protocol path is not ported; this device is unsupported",
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
        Err(err("Lian Li module is not initialized"))
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
