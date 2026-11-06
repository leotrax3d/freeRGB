// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};
pub const MODULE_ID: &str = "corsair-lighting-node";
/// upstream: CorsairLightingNodeController.cpp SetChannelLEDs sends 50 LEDs per packet
pub const LED_PER_PACKET: usize = 50;
/// upstream: RGBController_CorsairLightingNode.cpp NUM_CHANNEL constant
pub const NUM_CHANNELS: usize = 2;
#[derive(Default)]
pub struct N {
    counts: Vec<usize>,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for N {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CorsairLightingNodeController".into(),
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
        self.initialize_with_zones(i, None, h)
    }
    fn initialize_with_zones(
        &mut self,
        i: &InterfaceInfo,
        z: Option<&[Zone]>,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        table::device(i).ok_or_else(|| err("unsupported Corsair Lighting Node"))?;
        // upstream: RGBController_CorsairLightingNode.cpp SetupZones resizes to
        // two "Corsair RGB Header" zones with leds_max 204, default 0; the module
        // needs at least one LED per zone, so the default is one.
        self.counts = z
            .map(|x| x.iter().map(|q| q.leds.clamp(0, 204)).collect())
            .unwrap_or_else(|| vec![1; NUM_CHANNELS]);
        if self.counts.len() != NUM_CHANNELS {
            return Err(err("Corsair Lighting Node requires two zones"));
        }
        self.active = true;
        let leds: usize = self.counts.iter().sum();
        let mut led = Vec::new();
        for (n, &count) in self.counts.iter().enumerate() {
            let name = format!("Corsair RGB Header {}", n + 1);
            for k in 0..count {
                led.push(Led {
                    name: format!("{}, LED {}", name, k + 1),
                    pos: ((k % 50).min(u16::MAX as usize) as u16, n as u16),
                });
            }
        }
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Corsair Lighting Node".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: leds.min(u16::MAX as usize) as u16,
                height: NUM_CHANNELS as u16,
                leds: led,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: self
                .counts
                .iter()
                .enumerate()
                .map(|(n, &leds)| Zone {
                    name: format!("Corsair RGB Header {}", n + 1),
                    leds,
                })
                .collect(),
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.counts.iter().sum::<usize>() {
            return Err(err("invalid Lighting Node frame"));
        }
        // upstream: CorsairLightingNodeController.cpp SetChannelLEDs: PortState
        // per channel, then per 50-LED block the R, G and B plane packets, then
        // one Commit per channel.
        let mut at = 0;
        for (ch, &n) in self.counts.iter().enumerate() {
            h.write(&report::state(ch as u8)).map_err(err)?;
            let mut pkt_offset = 0u8;
            for chunk in c[at..at + n].chunks(LED_PER_PACKET) {
                for plane in 0..3u8 {
                    h.write(&report::direct(ch as u8, pkt_offset, chunk, plane))
                        .map_err(err)?;
                }
                pkt_offset += chunk.len() as u8;
            }
            h.write(&report::commit()).map_err(err)?;
            at += n;
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
