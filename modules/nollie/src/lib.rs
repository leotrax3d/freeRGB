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
pub const MODULE_ID: &str = "nollie";
#[derive(Default)]
pub struct Nollie {
    pid: u16,
    channels: Vec<usize>,
    protocol: &'static str,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for Nollie {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "NollieController".into(),
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
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::device(i).ok_or_else(|| err("unsupported Nollie device"))?;
        self.pid = d.pid;
        self.protocol = d.protocol;
        self.channels = z
            .map(|x| x.iter().map(|q| q.leds.clamp(0, 1024)).collect())
            .unwrap_or_else(|| vec![d.leds; d.channels]);
        if self.channels.len() != d.channels {
            return Err(err("Nollie zone count does not match device channels"));
        }
        self.active = true;
        let total: usize = self.channels.iter().sum();
        let mut leds = Vec::new();
        let mut zones = Vec::new();
        let mut off = 0;
        for (c, n) in self.channels.iter().enumerate() {
            zones.push(Zone {
                name: format!("Channel {}", c + 1),
                leds: *n,
            });
            for k in 0..*n {
                leds.push(Led {
                    name: format!("Channel {} LED {}", c + 1, k),
                    pos: (k as u16, c as u16),
                });
            }
            off += n;
        }
        let _ = off;
        Ok(Capabilities {
            name: table::identify(i).model.unwrap_or_else(|| "Nollie".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: total.min(u16::MAX as usize) as u16,
                height: d.channels as u16,
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
        if !self.active {
            return Err(err("Nollie module is not initialized"));
        }
        if c.len() != self.channels.iter().sum::<usize>() {
            return Err(err("invalid Nollie frame"));
        }
        let mut off = 0;
        for (ch, n) in self.channels.iter().enumerate() {
            let colors = &c[off..off + *n];
            if self.protocol == "high" {
                let flag = if ch == 15 {
                    1
                } else if ch == 31 {
                    2
                } else {
                    0
                };
                h.write(&report::high_speed(ch, colors, flag))
                    .map_err(err)?;
            } else {
                for (chunk_idx, chunk) in colors.chunks(21).enumerate() {
                    h.write(&report::fixed(self.pid, ch, chunk_idx, chunk))
                        .map_err(err)?;
                }
                h.write(&report::update()).map_err(err)?;
            }
            off += *n;
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
    fn catalog_has_16_detectors() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 16);
    }
}
