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
pub const MODULE_ID: &str = "lianli";
#[derive(Default)]
pub struct L {
    pid: u16,
    leds: usize,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for L {
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
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::device(i).ok_or_else(|| err("unsupported Lian Li HID"))?;
        self.pid = d.pid;
        self.leds = d.leds;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Lian Li HID".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 20,
                height: 6,
                leds: (0..self.leds)
                    .map(|n| Led {
                        name: format!("LED {n}"),
                        pos: ((n % 20) as u16, (n / 20) as u16),
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
            return Err(err("invalid Lian Li frame"));
        }
        let ch = if self.pid == 0xA200 { 1 } else { 0x40 };
        for p in report::colors(ch, c) {
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
        assert_eq!(crate::devices_table::DETECTORS.len(), 9);
    }
}
