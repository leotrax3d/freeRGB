// SPDX-License-Identifier: GPL-2.0-or-later
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, MODULE_API,
};
pub const MODULE_ID: &str = "nzxt-mouse";
#[derive(Default)]
pub struct NzxtMouse {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for NzxtMouse {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "NZXTMouseController".into(),
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
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }
        h.write(&report::firmware_request()).map_err(err)?;
        for _ in 0..20usize {
            match h.read(64, 1000) {
                Ok(v) if v.len() >= 2 && v[0] == 0x43 && v[1] == 0x86 => break,
                _ => continue,
            }
        }
        self.active = true;
        let leds = (0..6usize)
            .map(|n| Led {
                name: format!("LED {}", n + 1),
                pos: (n as u16, 0),
            })
            .collect();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "NZXT Mouse".into()),
            kind: DeviceKind::Mouse,
            layout: LedLayout {
                width: 6,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 6 {
            return Err(err("invalid frame"));
        }
        h.write(&report::direct(colors)).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
