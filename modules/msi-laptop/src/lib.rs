// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod table;
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, MODULE_API,
};
pub const MODULE_ID: &str = "msi-laptop";
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
            family: "MSILaptopController".into(),
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
        Err(err("MSI laptop lifecycle is not ported"))
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        let _ = (c, h);
        Err(err("MSI laptop lifecycle is not ported"))
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn interface(d: &table::Detector) -> InterfaceInfo {
        InterfaceInfo {
            vid: d.vid,
            pid: d.pid,
            interface: d.interface,
            usage_page: d.usage_page,
            usage: d.usage,
            product: None,
        }
    }
    #[test]
    fn catalog() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 4);
    }
    #[test]
    fn inventoried_devices_are_not_claimed_before_layout_port() {
        let module = A;
        for d in devices_table::DETECTORS {
            let i = interface(d);
            assert!(!module.identify(&i).supported);
            assert!(table::device(&i).is_none());
        }
    }
}
