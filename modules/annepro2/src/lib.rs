// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod led_names;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, MODULE_API,
};
pub const MODULE_ID: &str = "annepro2";
/// upstream: RGBController_AnnePro2.cpp: LED_REAL_COUNT = 5*14, LED_COUNT = 61.
pub const REAL_LED_COUNT: usize = 5 * 14;
pub const LED_COUNT: usize = 61;
#[derive(Default)]
pub struct A {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for A {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "AnnePro2Controller".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        _: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        table::device(i).ok_or_else(|| err("unsupported Anne Pro 2"))?;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Anne Pro 2".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 14,
                height: 5,
                leds: (0..LED_COUNT)
                    .map(|n| Led {
                        name: crate::led_names::LED_NAMES[n].0.replace('_', " "),
                        pos: ((n % 14) as u16, (n / 14) as u16),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != LED_COUNT {
            return Err(err("invalid Anne Pro 2 frame"));
        }
        // upstream: RGBController_AnnePro2.cpp DeviceUpdateLEDs writes each LED's
        // color at its real frame index; indices 40, 41, 52, 53 and 60 consume one
        // extra (zero) triplet, indices 55 and 56 consume two extra triplets.
        let mut raw = vec![0u8; REAL_LED_COUNT * 3];
        let mut real = 0usize;
        for (i, color) in c.iter().enumerate() {
            raw[real * 3..real * 3 + 3].copy_from_slice(&[color.r, color.g, color.b]);
            real += 1;
            if matches!(i, 40 | 41 | 52 | 53 | 60) {
                real += 1;
            } else if matches!(i, 55 | 56) {
                real += 2;
            }
        }
        debug_assert_eq!(real, REAL_LED_COUNT);
        for p in report::direct(&raw) {
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
