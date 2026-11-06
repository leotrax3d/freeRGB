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
pub const MODULE_ID: &str = "asus-monitor";
#[derive(Default)]
pub struct M {
    leds: usize,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for M {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "AsusMonitorController".into(),
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
        table::device(i).ok_or_else(|| err("unsupported ASUS monitor"))?;
        // upstream: RGBController_AsusMonitor.cpp SetupZones reads the LED count
        // via GetNumberOfLEDs (query 0xEC 0xB0, count in response byte 32); a
        // device that does not answer is not configured.
        h.write(&report::query()).map_err(err)?;
        let r = h.read(65, 100).map_err(err)?;
        if r.len() <= 32 || r[32] == 0 {
            return Err(err("ASUS monitor did not report an LED count"));
        }
        self.leds = usize::from(r[32]);
        h.write(&report::init()).map_err(err)?;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "ASUS Monitor".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: self.leds as u16,
                height: 1,
                leds: (0..self.leds)
                    .map(|n| Led {
                        name: format!("LED {n}"),
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
            return Err(err("invalid ASUS monitor frame"));
        }
        h.write(&report::direct(c)).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
