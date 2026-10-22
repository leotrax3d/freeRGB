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
pub const MODULE_ID: &str = "sinowealth";
#[derive(Default)]
pub struct Sinowealth {
    kind: &'static str,
    leds: usize,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Sinowealth {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "SinowealthController".into(),
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
        let d = table::device(i).ok_or_else(|| err("unsupported Sinowealth device"))?;
        self.kind = d.kind;
        self.leds = d.leds;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Sinowealth".into()),
            kind: if self.kind == "mouse" {
                DeviceKind::Mouse
            } else {
                DeviceKind::Keyboard
            },
            layout: LedLayout {
                width: 21,
                height: 6,
                leds: (0..self.leds)
                    .map(|n| Led {
                        name: format!("LED {n}"),
                        pos: ((n % 21) as u16, (n / 21) as u16),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.leds {
            return Err(err("invalid Sinowealth frame"));
        }
        if self.kind == "mouse" {
            h.write(&report::mouse(c, 1)).map_err(err)?;
        } else {
            for x in c.chunks(16) {
                h.write(&report::direct(x)).map_err(err)?;
            }
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
    fn catalog_has_13_detectors() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 13);
    }
}
