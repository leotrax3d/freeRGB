// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod magic;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};
pub const MODULE_ID: &str = "corsair-hydro-platinum";
#[derive(Default)]
pub struct Hydro {
    active: bool,
    sequence: u8,
    rgb_fan: bool,
    leds: usize,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Hydro {
    /// upstream: CorsairHydroPlatinumController.cpp GetSequenceNumber: an atomic
    /// counter starting at 0, incremented to at most 31 (then wrapping to 1) on
    /// every packet; the packet carries (sequence << 3) | command.
    fn next_sequence(&mut self) -> u8 {
        self.sequence = if self.sequence >= 31 {
            1
        } else {
            self.sequence + 1
        };
        self.sequence
    }
    fn send_magic(
        &mut self,
        data: &[u8; 61],
        command: u8,
        h: &mut dyn Hid,
    ) -> Result<(), ModuleError> {
        h.write(&report::magic(data, command, self.next_sequence()))
            .map_err(err)?;
        Ok(())
    }
}
impl Module for Hydro {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CorsairHydroPlatinumController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize_with_zones(
        &mut self,
        i: &InterfaceInfo,
        z: Option<&[Zone]>,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::device(i).ok_or_else(|| err("unsupported Corsair Hydro device"))?;
        self.rgb_fan = d.rgb_fan;
        self.sequence = 0;
        self.send_magic(magic::MAGIC_1, 1, h)?;
        self.send_magic(magic::MAGIC_2, 2, h)?;
        self.send_magic(magic::MAGIC_3, 3, h)?;
        // upstream: RGBController_CorsairHydroPlatinum.cpp SetupZones -
        // fixed 16-LED 5x5 CPU block, plus a 0..32-LED fan zone only on
        // models that support an RGB fan.
        let channels: Vec<usize> = match z {
            Some(z) => {
                if z.is_empty() {
                    vec![16]
                } else {
                    let mut c: Vec<usize> = z.iter().map(|q| q.leds.clamp(0, 32)).collect();
                    c[0] = 16;
                    c
                }
            }
            None => vec![16],
        };
        if !self.rgb_fan && channels.len() > 1 {
            return Err(err("this Corsair Hydro model has no RGB fan header"));
        }
        if channels[0] != 16 {
            return Err(err("Corsair Hydro CPU block must have 16 LEDs"));
        }
        self.leds = channels.iter().sum();
        self.active = true;
        let total = self.leds;
        let mut zones = Vec::new();
        let mut leds = Vec::new();
        for (zn, n) in channels.iter().enumerate() {
            let name = if zn == 0 {
                "CPU Block".to_string()
            } else {
                "Corsair Fan Header".to_string()
            };
            zones.push(Zone {
                name: name.clone(),
                leds: *n,
            });
            for k in 0..*n {
                let led_name = format!("{}, LED {}", name, k + 1);
                // Zone 0 is a 5x5 matrix; the fan zone is linear.
                let pos = if zn == 0 {
                    ((k % 5) as u16, (k / 5) as u16)
                } else {
                    (k as u16, 1)
                };
                leds.push(Led {
                    name: led_name,
                    pos,
                });
            }
        }
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Corsair Hydro Platinum".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: total.min(u16::MAX as usize) as u16,
                height: 1,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones,
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
        if !self.active || c.len() != self.leds {
            return Err(err("invalid Corsair Hydro frame"));
        }
        // upstream: CorsairHydroPlatinumController.cpp SetupColors sends the
        // combined CPU block + fan colors in fixed 20-LED chunks with commands
        // SET_LIGHTING_1/2/3, capped at 48 LEDs.
        let stops = [20usize, 40, 48];
        let mut start = 0usize;
        for (k, &stop) in stops.iter().enumerate() {
            let cmd = 4 + k as u8;
            let end = c.len().min(stop);
            if start >= end {
                break;
            }
            let seq = self.next_sequence();
            h.write(&report::colors(&c[start..end], cmd, seq))
                .map_err(err)?;
            start = stop;
        }
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, h: &mut dyn Hid) -> Result<(), ModuleError> {
        // upstream: CorsairHydroPlatinumController.cpp destructor restores
        // hardware lighting with HARDWARE_MAGIC_2/3/4/1 and commands 2/3/4/1.
        self.send_magic(magic::HARDWARE_MAGIC_2, 2, h)?;
        self.send_magic(magic::HARDWARE_MAGIC_3, 3, h)?;
        self.send_magic(magic::HARDWARE_MAGIC_4, 4, h)?;
        self.send_magic(magic::HARDWARE_MAGIC_1, 1, h)?;
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
