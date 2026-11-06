// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod table;
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, MODULE_API,
};
pub const MODULE_ID: &str = "msi-keyboard";
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
            family: "MSIKeyboardController".into(),
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
        Err(err("MSI keyboard lifecycle is not ported"))
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        let _ = (c, h);
        Err(err("MSI keyboard lifecycle is not ported"))
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn interface(detector: &table::Detector) -> InterfaceInfo {
        InterfaceInfo {
            vid: detector.vid,
            pid: detector.pid,
            interface: detector.interface,
            usage_page: detector.usage_page,
            usage: detector.usage,
            product: None,
        }
    }
    #[test]
    fn catalog() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 4);
    }

    #[test]
    fn inventoried_devices_are_not_claimed_before_controller_port() {
        let module = A;
        for detector in devices_table::DETECTORS {
            let i = interface(detector);
            assert!(
                !module.identify(&i).supported,
                "{} must remain unsupported",
                detector.name
            );
            assert!(table::device(&i).is_none());
        }
    }
}
