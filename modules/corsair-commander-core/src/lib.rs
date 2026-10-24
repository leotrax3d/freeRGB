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
pub const MODULE_ID: &str = "corsair-commander-core";
#[derive(Default)]
pub struct C {
    counts: Vec<usize>,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for C {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CorsairCommanderCoreController".into(),
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
        _h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        table::device(i).ok_or_else(|| err("unsupported Corsair Commander Core"))?;
        self.counts = z
            .map(|x| x.iter().map(|q| q.leds).collect())
            .unwrap_or_else(|| vec![24, 34, 34, 34, 34, 34, 34]);
        if self.counts.len() != 7 {
            return Err(err("Commander Core requires seven zones"));
        }
        self.active = true;
        let leds = self.counts.iter().sum();
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Corsair Commander Core".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: 34,
                height: 7,
                leds: (0..leds)
                    .map(|n| Led {
                        name: format!("LED {n}"),
                        pos: ((n % 34) as u16, (n / 34) as u16),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: self
                .counts
                .iter()
                .enumerate()
                .map(|(n, &leds)| Zone {
                    name: format!("Channel {n}"),
                    leds,
                })
                .collect(),
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.counts.iter().sum::<usize>() {
            return Err(err("invalid Commander Core frame"));
        }
        let mut at = 0;
        for (ch, &n) in self.counts.iter().enumerate() {
            for p in report::packet(ch as u8, &c[at..at + n]).chunks(97) {
                h.write(p).map_err(err)?;
            }
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
mod tests {
    #[test]
    fn catalog() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 7);
    }
}
