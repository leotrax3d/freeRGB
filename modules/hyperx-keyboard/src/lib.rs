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
pub const MODULE_ID: &str = "hyperx-keyboard";
#[derive(Default)]
pub struct HyperX {
    kind: &'static str,
    leds: usize,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for HyperX {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "HyperXKeyboardController".into(),
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
        let d = table::device(i).ok_or_else(|| err("unsupported HyperX device"))?;
        self.kind = d.kind;
        self.leds = d.leds;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "HyperX Keyboard".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 21,
                height: 6,
                leds: (0..self.leds)
                    .map(|n| Led {
                        name: format!("Keyboard LED {n}"),
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
            return Err(err("invalid HyperX frame"));
        }
        match self.kind {
            "origins" => {
                h.write(&report::origins_init(if self.leds <= 67 { 5 } else { 9 }))
                    .map_err(err)?;
                for x in c.chunks(16) {
                    h.write(&report::origins(x)).map_err(err)?;
                }
            }
            "elite2" => {
                h.write(&report::elite2_init()).map_err(err)?;
                for p in report::elite2(c) {
                    h.write(&p).map_err(err)?;
                }
            }
            "origins2" => {
                for (i, x) in c.chunks(20).enumerate() {
                    h.write(&report::origins2(i as u8, x)).map_err(err)?;
                }
            }
            "eve" => {
                for x in c.chunks(20) {
                    h.write(&report::eve(x)).map_err(err)?;
                }
            }
            "origins-core" => {
                let bytes = c.chunks(20);
                for (i, x) in bytes.enumerate() {
                    h.write(&report::core(x, i as u8)).map_err(err)?;
                }
            }
            "fps" => {
                for ch in [0x01_u8, 0x02, 0x03] {
                    h.write(&report::channel(c, ch, ch)).map_err(err)?;
                }
            }
            "elite" => {
                for ch in [0x01_u8, 0x02, 0x03] {
                    h.write(&report::channel(c, ch, ch)).map_err(err)?;
                }
                h.write(&report::extended(c)).map_err(err)?;
            }
            _ => {
                for (i, x) in c.chunks(16).enumerate() {
                    h.write(&report::origins2(i as u8, x)).map_err(err)?;
                }
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
    fn catalog_has_20_detectors() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 20);
    }
}
