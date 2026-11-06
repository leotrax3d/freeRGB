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
pub const MODULE_ID: &str = "nzxt-hue1";

#[derive(Default)]
pub struct M {
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
            family: "NZXTHue1Controller".into(),
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
        table::device(i).ok_or_else(|| err("unsupported NZXT Hue 1 device"))?;
        // upstream: NZXTHue1Controller.cpp Initialize sends `01 5C`,
        // then `01 5D`, then reads 21-byte status packets until one arrives.
        // The first branch version sent the query and read; a device that
        // does not answer is not configured (loop would never end upstream).
        h.write(&report::init()).map_err(err)?;
        h.write(&report::start_reporting()).map_err(err)?;
        let r = h.read(21, 100).map_err(err)?;
        if r.len() <= 0x11 {
            return Err(err("NZXT Smart Device V1 did not report a status packet"));
        }
        // upstream: byte 0x11 = device count, (byte 0x10 >> 3) = accessory
        // type (0: Hue+ strip 10 LEDs per device, else Aer fan 8 LEDs).
        let dev_count = r[0x11];
        let accessory = r[0x10] >> 3;
        let per_device = if accessory == 0 { 10 } else { 8 };
        let leds = usize::from(dev_count) * per_device;
        if leds == 0 {
            return Err(err("NZXT Smart Device V1 reported 0 connected devices"));
        }
        // The Direct-mode packet path (two 65-byte writes) carries at most 40
        // colors; larger counts would exceed the upstream 120-byte color_data
        // buffer, so they fail closed.
        if leds > report::MAX_COLORS {
            return Err(err(
                "NZXT Hue 1 LED count exceeds the Direct-mode packet capacity",
            ));
        }
        self.leds = leds;
        self.active = true;
        let firmware = if r.len() > 0x0e {
            format!("{}.{}", r[0x0b], r[0x0e])
        } else {
            String::new()
        };
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "NZXT Hue 1".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: self.leds as u16,
                height: 1,
                leds: (0..self.leds)
                    .map(|n| Led {
                        name: format!("Hue 1 Channel, LED {}", n + 1),
                        pos: (n as u16, 0),
                    })
                    .collect(),
            },
            firmware: Some(firmware),
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.leds {
            return Err(err("invalid NZXT Hue 1 frame"));
        }
        for p in report::direct(c) {
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
