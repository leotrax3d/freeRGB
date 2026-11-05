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

pub const MODULE_ID: &str = "intel-arc-a770-le";

#[derive(Default)]
pub struct IntelArcA770Le {
    active: bool,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

impl Module for IntelArcA770Le {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "IntelArcA770LEController".into(),
            api: MODULE_API,
        }
    }

    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }

    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }

        let query = report::firmware_query();
        hid.write(&query).map_err(err)?;
        let response = hid.read(64, 1000).map_err(err)?;
        let firmware = report::parse_firmware(&response);

        hid.write(&report::enable()).map_err(err)?;
        let _ = hid.read(64, 1000).map_err(err)?;
        hid.write(&report::apply()).map_err(err)?;
        let _ = hid.read(64, 1000).map_err(err)?;

        self.active = true;

        let mut leds = Vec::with_capacity(report::LED_COUNT);
        for n in 0..report::LED_COUNT {
            let name = match n {
                0..=15 => format!("Fan 1 LED {}", n + 1),
                16..=31 => format!("Fan 2 LED {}", n - 15),
                32..=39 => format!("Back LED {}", n - 31),
                40..=89 => format!("Ring LED {}", n - 39),
                _ => "Logo LED 1".into(),
            };
            leds.push(Led {
                name,
                pos: (n as u16, 0),
            });
        }

        Ok(Capabilities {
            name: id
                .model
                .unwrap_or_else(|| "Intel Arc A770 Limited Edition".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: 91,
                height: 1,
                leds,
            },
            firmware: Some(firmware),
            restore: Restore::LastFrame,
            zones: vec![
                Zone {
                    name: "Fan 1".into(),
                    leds: 16,
                },
                Zone {
                    name: "Fan 2".into(),
                    leds: 16,
                },
                Zone {
                    name: "Back".into(),
                    leds: 8,
                },
                Zone {
                    name: "Ring".into(),
                    leds: 50,
                },
                Zone {
                    name: "Logo".into(),
                    leds: 1,
                },
            ],
        })
    }

    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != report::LED_COUNT {
            return Err(err("invalid Intel Arc A770 LE frame"));
        }
        for packet in report::frames(colors) {
            hid.write(&packet).map_err(err)?;
            let _ = hid.read(64, 250).map_err(err)?;
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
