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
pub const MODULE_ID: &str = "asus-aura-mouse";

#[derive(Default)]
pub struct M {
    pid: u16,
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
            family: "AsusAuraUSBController".into(),
            api: MODULE_API,
        }
    }

    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }

    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        _h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::device(i).ok_or_else(|| err("unsupported ASUS Aura mouse"))?;
        self.pid = d.pid;
        let (amount, slots) = report::led_map(d.pid);
        if amount == 0 || slots.is_empty() {
            return Err(err("no ASUS Aura Direct-mode LED map for this device"));
        }
        // upstream: RGBController_AsusAuraMouse.cpp SetupZones builds one
        // LED per zone named "<zone> LED" in zone id order; no init packets
        // exist (SendDirect works without any handshake).
        self.leds = slots.len();
        self.active = true;
        let leds: Vec<Led> = slots
            .iter()
            .map(|&(zone, _)| Led {
                name: format!("{} LED", table::ZONE_NAMES[zone as usize]),
                pos: (zone as u16, 0),
            })
            .collect();
        Ok(Capabilities {
            name: d.name.into(),
            kind: DeviceKind::Mouse,
            layout: LedLayout {
                width: self.leds as u16,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.leds {
            return Err(err("invalid ASUS Aura mouse frame"));
        }
        // upstream: AuraMouseController.cpp SendDirect paints every zone's
        // colour into all of its slots, then chunks 5 colours per packet.
        let (amount, slots) = report::led_map(self.pid);
        let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; amount as usize];
        for (zone_info, color) in slots.iter().zip(c.iter()) {
            for &slot in zone_info.1 {
                colors[slot] = *color;
            }
        }
        for p in report::direct(&colors) {
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
mod tests;
