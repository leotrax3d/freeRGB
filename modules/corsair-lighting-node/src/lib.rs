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
        self.counts = z
            .map(|x| x.iter().map(|q| q.leds).collect())
            .unwrap_or_else(|| vec![0, 0]);
        if self.counts.len() != 2 {
            return Err(err("Lighting Node requires two zones"));
        }
        self.active = true;
        let leds = self.counts.iter().sum();
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Corsair Lighting Node".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: 50,
                height: 2,
                leds: (0..leds)
                    .map(|n| Led {
                        name: format!("LED {n}"),
                        pos: ((n % 50) as u16, (n / 50) as u16),
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
                    name: format!("RGB Header {}", n + 1),
                    leds,
                })
                .collect(),
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.counts.iter().sum::<usize>() {
            return Err(err("invalid Lighting Node frame"));
        }
        let mut at = 0;
        for (ch, &n) in self.counts.iter().enumerate() {
            h.write(&report::state(ch as u8)).map_err(err)?;
            for plane in 0..3 {
                for part in c[at..at + n].chunks(50) {
                    h.write(&report::direct(ch as u8, 0, part, plane))
                        .map_err(err)?;
                }
            }
            at += n;
        }
        h.write(&report::commit()).map_err(err)?;
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
