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
pub const MODULE_ID: &str = "turtle-beach-kp7";

#[derive(Default)]
pub struct M {
    active: bool,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

/// Keypad LED names in zone/presentation order. Plain upstream strings for
/// the indicators; numpad key labels follow the upstream KEY_EN names.
const INDICATOR_NAMES: [&str; 3] = [
    "Num Lock Indicator",
    "Shift Mode Indicator",
    "Key: Shift Mode",
];
const KEYPAD_NAMES: [&str; 28] = [
    "F13",
    "F14",
    "F15",
    "Key: Profile Switch",
    "Numpad Lock",
    "Numpad /",
    "Numpad *",
    "Numpad -",
    "Key: Mode 1",
    "Numpad 7",
    "Numpad 8",
    "Numpad 9",
    "Numpad +",
    "Key: Mode 2",
    "Numpad 4",
    "Numpad 5",
    "Numpad 6",
    "Numpad +",
    "Key: Mode 3",
    "Numpad 1",
    "Numpad 2",
    "Numpad 3",
    "Numpad Enter",
    "Key: Mode 4",
    "Numpad 0",
    "Numpad .",
    "Numpad Enter",
    "Space",
];

/// Upstream: RGBController_TurtleBeachKP7.cpp SetupZones / matrix map.
/// Position [x, y] per keypad zone index; NA cells are skipped here.
const KEYPAD_MATRIX: [[i32; 5]; 7] = [
    [0, 1, 2, 3, NA],
    [4, 5, 6, 7, 8],
    [9, 10, 11, 12, 13],
    [14, 15, 16, 17, 18],
    [19, 20, 21, 22, 23],
    [24, NA, 25, 26, NA],
    [NA, NA, NA, 27, NA],
];
const NA: i32 = -1;

impl Module for M {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "TurtleBeachKP7Controller".into(),
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
        table::device(i).ok_or_else(|| err("unsupported Turtle Beach KP7 device"))?;
        // upstream: TurtleBeachKP7Controller constructor calls
        // EnterDirectMode; until this 5-byte feature report is sent the
        // keypad ignores everything written to the RGB reports.
        h.send_feature(&report::enter_direct()).map_err(err)?;
        self.active = true;
        let mut leds = Vec::with_capacity(report::LED_COUNT);
        for (n, name) in INDICATOR_NAMES.iter().enumerate() {
            leds.push(Led {
                name: (*name).into(),
                pos: (n as u16, 0),
            });
        }
        for (n, name) in KEYPAD_NAMES.iter().enumerate() {
            let pos = KEYPAD_MATRIX
                .iter()
                .enumerate()
                .find_map(|(row, cells)| {
                    cells
                        .iter()
                        .position(|c| *c == n as i32 && *c != NA)
                        .map(|col| (col as u16, (row + 1) as u16))
                })
                .unwrap_or((n as u16, 0));
            leds.push(Led {
                name: (*name).into(),
                pos,
            });
        }
        for n in 0..2usize {
            leds.push(Led {
                name: format!("Wrist Rest {}", n + 1),
                pos: (n as u16, 8),
            });
        }
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "Turtle Beach Command Series KP7".into()),
            kind: DeviceKind::Keypad,
            layout: LedLayout {
                width: 5,
                height: 9,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![
                freergb_module_api::Zone {
                    name: "Indicators".into(),
                    leds: 3,
                },
                freergb_module_api::Zone {
                    name: "Keypad".into(),
                    leds: 28,
                },
                freergb_module_api::Zone {
                    name: "Wrist Rest".into(),
                    leds: 2,
                },
            ],
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != report::LED_COUNT {
            return Err(err("invalid Turtle Beach KP7 frame"));
        }
        for p in report::frame(c) {
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
