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
pub const MODULE_ID: &str = "xpg-summoner-keyboard";
static LED_VALUES: [usize; 104] = [
    11, 22, 30, 25, 27, 7, 51, 57, 62, 86, 87, 83, 85, 79, 72, 0, 14, 15, 23, 31, 39, 38, 46, 47,
    55, 63, 71, 70, 54, 81, 102, 118, 110, 92, 100, 108, 109, 9, 8, 16, 24, 32, 33, 41, 40, 48, 56,
    64, 65, 49, 82, 94, 119, 111, 88, 96, 104, 112, 17, 10, 18, 26, 34, 35, 43, 42, 50, 58, 66, 67,
    84, 89, 97, 105, 121, 12, 20, 28, 36, 37, 45, 44, 52, 60, 69, 122, 115, 90, 98, 106, 114, 6,
    124, 75, 91, 77, 125, 61, 4, 117, 93, 101, 99, 107,
];
#[derive(Default)]
pub struct XpgSummonerKeyboard {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for XpgSummonerKeyboard {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "XPGSummonerKeyboardController".into(),
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
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }
        self.active = true;
        h.write(&report::init_report()).map_err(err)?;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "XPG Summoner".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 104,
                height: 1,
                leds: (0..104)
                    .map(|i| Led {
                        name: format!("LED {}", i),
                        pos: (i as u16, 0),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 104 {
            return Err(err("invalid XPG Summoner frame"));
        }
        let mut frame_buf = [0u8; 504];
        for (linear_idx, c) in colors.iter().enumerate() {
            let real_idx = LED_VALUES[linear_idx];
            frame_buf[real_idx * 4] = 0x64;
            frame_buf[real_idx * 4 + 1] = c.r;
            frame_buf[real_idx * 4 + 2] = c.g;
            frame_buf[real_idx * 4 + 3] = c.b;
        }
        h.write(&report::chunk(0, &frame_buf[0..256]))
            .map_err(err)?;
        h.write(&report::chunk(1, &frame_buf[256..])).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
