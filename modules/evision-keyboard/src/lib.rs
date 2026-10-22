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
pub const MODULE_ID: &str = "evision-keyboard";
#[derive(Default)]
pub struct EVision {
    v2: bool,
    active: bool,
    leds: usize,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for EVision {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "EVisionKeyboardController".into(),
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
        let d = table::device(i).ok_or_else(|| err("unsupported EVision device"))?;
        self.v2 = d.v2;
        self.leds = if self.v2 && d.pid == 0x0012 {
            104
        } else if self.v2 {
            106
        } else {
            126
        };
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "EVision Keyboard".into()),
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
            return Err(err("invalid EVision frame"));
        }
        let bytes: Vec<u8> = c.iter().flat_map(|x| [x.r, x.g, x.b]).collect();
        if self.v2 {
            for (off, chunk) in bytes.chunks(56).enumerate() {
                h.write(&report::v2(0x12, (off * 56) as u16, chunk))
                    .map_err(err)?;
            }
            h.write(&report::v2(0x13, (self.leds * 3) as u16, &[]))
                .map_err(err)?;
        } else {
            h.write(&report::v1_begin()).map_err(err)?;
            for (off, chunk) in bytes.chunks(0x36).enumerate() {
                h.write(&report::v1_colors(chunk, off * 0x36))
                    .map_err(err)?;
            }
            h.write(&report::v1_end()).map_err(err)?;
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
    fn catalog_has_15_detectors() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 15);
    }
}
