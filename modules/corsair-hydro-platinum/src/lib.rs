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
    channels: Vec<usize>,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
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
        table::device(i).ok_or_else(|| err("unsupported Corsair Hydro device"))?;
        for (data, command) in [
            (magic::MAGIC_1, 1_u8),
            (magic::MAGIC_2, 2),
            (magic::MAGIC_3, 3),
        ] {
            h.write(&report::magic(data, command, command))
                .map_err(err)?;
        }
        self.channels = z
            .map(|x| x.iter().map(|q| q.leds.clamp(0, 32)).collect())
            .unwrap_or_else(|| vec![16]);
        if self.channels.is_empty() || self.channels[0] != 16 {
            return Err(err("Corsair Hydro CPU block must have 16 LEDs"));
        }
        self.active = true;
        let total: usize = self.channels.iter().sum();
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Corsair Hydro Platinum".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: 5,
                height: 5,
                leds: (0..total)
                    .map(|n| Led {
                        name: format!("Hydro LED {n}"),
                        pos: ((n % 5) as u16, (n / 5) as u16),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: self
                .channels
                .iter()
                .enumerate()
                .map(|(i, n)| Zone {
                    name: if i == 0 {
                        "CPU Block".into()
                    } else {
                        "Corsair Fan Header".into()
                    },
                    leds: *n,
                })
                .collect(),
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
        if !self.active || c.len() != self.channels.iter().sum::<usize>() {
            return Err(err("invalid Corsair Hydro frame"));
        }
        let mut off = 0;
        for (i, n) in self.channels.iter().enumerate() {
            for (chunk, j) in c[off..off + *n].chunks(20).zip(0..) {
                h.write(&report::colors(chunk, 4 + i as u8 + j, 1 + j))
                    .map_err(err)?;
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
    fn catalog_has_13_detectors() {
        assert_eq!(crate::devices_table::DETECTORS.len(), 13);
    }
}
