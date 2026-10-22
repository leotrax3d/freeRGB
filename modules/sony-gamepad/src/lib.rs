// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod table;
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, MODULE_API,
};
pub const MODULE_ID: &str = "sony-gamepad";
#[derive(Default)]
pub struct A;
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for A {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "SonyGamepadController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let _ = i;
        Err(err("Sony gamepad lifecycle is not ported"))
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        let _ = (c, h);
        Err(err("Sony gamepad lifecycle is not ported"))
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn interface(pid: u16) -> InterfaceInfo {
        InterfaceInfo {
            vid: 0x054c,
            pid,
            interface: 0,
            usage_page: 0,
            usage: 0,
            product: None,
        }
    }

    #[test]
    fn catalog() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 5);
    }

    #[test]
    fn inventoried_devices_are_not_claimed_before_transport_port() {
        let module = A;
        for detector in devices_table::DETECTORS {
            let id = module.identify(&interface(detector.pid));
            assert!(!id.supported, "{} must remain unsupported", detector.name);
            assert!(table::device(&interface(detector.pid)).is_none());
        }
    }
}
