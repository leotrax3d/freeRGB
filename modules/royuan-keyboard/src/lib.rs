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
pub const MODULE_ID: &str = "royuan-keyboard";
#[derive(Default)]
pub struct A {
    active: bool,
    protocol: table::Protocol,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for A {
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
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let device = table::device(i).ok_or_else(|| err("unsupported Royuan keyboard"))?;
        self.protocol = device.protocol;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Anne Pro 2".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: if device.protocol == table::Protocol::Skyloong {
                    15
                } else {
                    1
                },
                height: if device.protocol == table::Protocol::Skyloong {
                    5
                } else {
                    1
                },
                leds: (0..device.leds)
                    .map(|n| Led {
                        name: format!("LED {n}"),
                        pos: ((n % 14) as u16, (n / 14) as u16),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != table::device_count(c) {
            return Err(err("invalid Royuan keyboard frame"));
        }
        for p in report::frames(c, self.protocol) {
            h.write(&p).map_err(err)?;
        }
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn catalog() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 5);
    }
}
