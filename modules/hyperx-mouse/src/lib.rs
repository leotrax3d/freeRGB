// SPDX-License-Identifier: GPL-2.0-or-later
//! HyperX mouse module. The Pulsefire Surge controller path is fully ported
//! (264-byte feature reports, 32-LED RGB planes + logo LED); the FPS Pro /
//! Dart / Raid / Haste paths stay unsupported. The upstream 50 ms RGB update
//! keepalive thread is not portable to the frame-driven module API.
pub mod devices_table;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::Hid;
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};
pub const MODULE_ID: &str = "hyperx-mouse";
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
#[derive(Default)]
pub struct HyperX {
    active: bool,
    leds: usize,
}
impl freergb_module_api::sdk::Module for HyperX {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "HyperXMouseController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize_with_zones(
        &mut self,
        i: &InterfaceInfo,
        _: Option<&[freergb_module_api::Zone]>,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::detector(i).ok_or_else(|| err("no HyperX mouse detector"))?;
        let dev = crate::devices_table::DEVICES
            .iter()
            .find(|dev| dev.pid == d.pid && dev.protocol == "surge")
            .ok_or_else(|| err("HyperX mouse controller path not ported"))?;
        // upstream: RGBController_HyperXPulsefireSurge.cpp SetupZones()
        // (LED Strip 32 + Logo 1; constructor sends nothing)
        self.leds = dev.leds;
        self.active = true;
        let count = self.leds;
        Ok(Capabilities {
            name: d.name.into(),
            kind: DeviceKind::Mouse,
            layout: LedLayout {
                width: count as u16,
                height: 1,
                leds: (0..count)
                    .map(|n| Led {
                        name: if n < 32 {
                            format!("LED Strip LED {}", n + 1)
                        } else {
                            "Logo".into()
                        },
                        pos: (n as u16, 0),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![
                Zone {
                    name: "LED Strip".into(),
                    leds: 32,
                },
                Zone {
                    name: "Logo".into(),
                    leds: 1,
                },
            ],
        })
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, h)
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active {
            return Err(err("HyperX mouse module is not initialized"));
        }
        if c.len() != self.leds {
            return Err(err("invalid HyperX frame"));
        }
        // upstream: DeviceUpdateLEDs() -> HyperXPulsefireSurgeController
        // SendDirect(): one 264-byte feature report per frame
        h.send_feature(&report::surge_direct(c)).map_err(err)?;
        Ok(())
    }
    fn shutdown(
        &mut self,
        _: Option<freergb_core::Rgb>,
        _: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
