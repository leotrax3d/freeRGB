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
pub const MODULE_ID: &str = "thermaltake-poseidon-z-rgb";
#[derive(Default)]
pub struct ThermaltakePoseidonZRgb {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
const MATRIX_MAP: [[i32; 23]; 6] = [
    [
        0, -1, 8, 15, 22, 29, -1, 37, 44, 51, 58, -1, 65, 73, 81, 88, 94, 100, 102, -1, -1, -1, -1,
    ],
    [
        1, 9, 16, 23, 30, 38, 45, 52, 59, 66, 74, -1, 82, 89, 103, -1, 7, 21, 36, 50, 64, 80, 93,
    ],
    [
        2, -1, 10, 17, 24, 31, -1, 39, 46, 53, 60, 67, 75, 83, 90, 95, 14, 28, 43, 57, 72, 87, 86,
    ],
    [
        3, -1, 11, 18, 25, 32, -1, 40, 47, 54, 61, 68, 76, 84, 96, -1, -1, -1, -1, 35, 99, 63, -1,
    ],
    [
        4, -1, 26, 33, 41, 48, -1, 55, -1, 62, 69, 77, 85, 91, 101, -1, -1, 27, -1, 42, 49, 71, 98,
    ],
    [
        5, 12, 19, -1, -1, -1, -1, 34, -1, -1, -1, -1, 70, 78, 92, 97, 6, 13, 20, 56, -1, 79, -1,
    ],
];
impl Module for ThermaltakePoseidonZRgb {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "ThermaltakePoseidonZRGBController".into(),
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
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }
        self.active = true;
        let mut pos_of = vec![(0u16, 0u16); 104];
        for row in 0..6usize {
            for col in 0..23usize {
                let led = MATRIX_MAP[row][col];
                if led >= 0 && (led as usize) < 104 {
                    pos_of[led as usize] = (col as u16, row as u16);
                }
            }
        }
        Ok(Capabilities {
            name: id
                .model
                .unwrap_or_else(|| "Thermaltake Poseidon Z RGB".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 23,
                height: 6,
                leds: (0..104)
                    .map(|i| Led {
                        name: format!("LED {}", i + 1),
                        pos: pos_of[i],
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![freergb_module_api::Zone {
                name: "Keyboard".into(),
                leds: 104,
            }],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 104 {
            return Err(err("invalid Thermaltake Poseidon Z RGB frame"));
        }
        h.send_feature(&report::red_grn(colors)).map_err(err)?;
        h.send_feature(&report::blue(colors)).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
