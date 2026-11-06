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
pub const MODULE_ID: &str = "zalman-z-sync";
/// upstream: RGBController_ZalmanZSync.cpp SetupZones resizes to
/// ZALMAN_Z_SYNC_NUM_CHANNELS zones with leds_max 40, default 0.
pub const NUM_CHANNELS: usize = 8;
pub const MAX_LEDS_PER_CHANNEL: usize = 40;

#[derive(Default)]
pub struct M {
    counts: Vec<usize>,
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
            family: "ZalmanZSyncController".into(),
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
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        table::device(i).ok_or_else(|| err("unsupported Zalman Z Sync"))?;
        // upstream: RGBController_ZalmanZSync.cpp zones default to 0 LEDs
        // and are manually configurable; the module needs at least one LED
        // per zone, so the default is one. Zone sizes clamp to leds_max 40.
        self.counts = z
            .map(|x| {
                x.iter()
                    .map(|q| q.leds.clamp(0, MAX_LEDS_PER_CHANNEL))
                    .collect()
            })
            .unwrap_or_else(|| vec![1; NUM_CHANNELS]);
        if self.counts.len() != NUM_CHANNELS {
            return Err(err("Zalman Z Sync requires eight zones"));
        }
        self.active = true;
        let leds: usize = self.counts.iter().sum();
        let mut led = Vec::new();
        for (n, &count) in self.counts.iter().enumerate() {
            let name = format!("Addressable RGB Header {}", n + 1);
            for k in 0..count {
                led.push(Led {
                    name: format!("{}, LED {}", name, k + 1),
                    pos: ((k % 50).min(u16::MAX as usize) as u16, n as u16),
                });
            }
        }
        // upstream: ZalmanZSyncController.cpp constructor calls
        // SendFirmwareRequest (`00 02` write + 17-byte read); a device that
        // does not answer is tolerated exactly as upstream.
        h.write(&report::firmware_request()).map_err(err)?;
        let _ = h.read(17, 100);
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Zalman Z Sync".into()),
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
                    name: format!("Addressable RGB Header {}", n + 1),
                    leds,
                })
                .collect(),
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.counts.iter().sum::<usize>() {
            return Err(err("invalid Zalman Z Sync frame"));
        }
        // upstream: ZalmanZSyncController.cpp SetChannelLEDs: PortState per
        // channel, then per 50-LED block the R, G and B plane packets, then
        // one Commit per channel.
        let mut at = 0;
        for (ch, &n) in self.counts.iter().enumerate() {
            h.write(&report::state(ch as u8)).map_err(err)?;
            let mut pkt_offset = 0u8;
            for chunk in c[at..at + n].chunks(report::LED_PER_PACKET) {
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
