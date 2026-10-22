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
pub const MODULE_ID: &str = "lenovo";
#[derive(Default)]
pub struct Lenovo {
    protocol: &'static str,
    leds: usize,
    pid: u16,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Lenovo {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "LenovoControllers".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::device(i).ok_or_else(|| err("unsupported Lenovo device"))?;
        self.protocol = d.protocol;
        self.leds = d.leds;
        self.pid = d.pid;
        self.active = true;
        if self.protocol == "legacy" {
            h.send_feature(&[7, 0xb2]).map_err(err)?;
        }
        Ok(Capabilities {
            name: table::identify(i).model.unwrap_or_else(|| "Lenovo".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: self.leds as u16,
                height: 1,
                leds: (0..self.leds)
                    .map(|n| Led {
                        name: format!("LED {}", n),
                        pos: (n as u16, 0),
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
            return Err(err("invalid Lenovo frame"));
        }
        match self.protocol {
            "legacy" => {
                for p in report::legacy(self.pid, c) {
                    h.send_feature(&p).map_err(err)?;
                }
            }
            "gen7-8" => {
                h.send_feature(&report::gen7(self.pid, c)).map_err(err)?;
            }
            "four-zone" => {
                h.send_feature(&report::four_zone(c)).map_err(err)?;
            }
            "m300" => {
                h.write(&report::m300(c[0])).map_err(err)?;
            }
            "k510" => {}
            "k500" => {
                for p in report::k500(c) {
                    h.write(&p).map_err(err)?;
                }
            }
            _ => {}
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
    fn catalog_has_all_detectors() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 24);
    }
}
