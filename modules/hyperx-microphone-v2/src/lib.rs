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
pub const MODULE_ID: &str = "hyperx-microphone-v2";
#[derive(Default)]
pub struct HyperXMicrophoneV2 {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
const MATRIX_MAP: [[u8; 12]; 9] = [
    [26, 27, 44, 45, 62, 63, 80, 81, 98, 99, 8, 9],
    [25, 28, 43, 46, 61, 64, 79, 82, 97, 100, 7, 10],
    [24, 29, 42, 47, 60, 65, 78, 83, 96, 101, 6, 11],
    [23, 30, 41, 48, 59, 66, 77, 84, 95, 102, 5, 12],
    [22, 31, 40, 49, 58, 67, 76, 85, 94, 103, 4, 13],
    [21, 32, 39, 50, 57, 68, 75, 86, 93, 104, 3, 14],
    [20, 33, 38, 51, 56, 69, 74, 87, 92, 105, 2, 15],
    [19, 34, 37, 52, 55, 70, 73, 88, 91, 106, 1, 16],
    [18, 35, 36, 53, 54, 71, 72, 89, 90, 107, 0, 17],
];
impl Module for HyperXMicrophoneV2 {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "HyperXMicrophoneV2Controller".into(),
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
        let mut pos_of = vec![(0u16, 0u16); 108];
        for row in 0..9usize {
            for col in 0..12usize {
                let led = MATRIX_MAP[row][col] as usize;
                if led < 108 {
                    pos_of[led] = (col as u16, row as u16);
                }
            }
        }
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "HyperX QuadCast 2 S".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: 12,
                height: 9,
                leds: (0..108)
                    .map(|i| freergb_core::Led {
                        name: format!("LED {}", i),
                        pos: pos_of[i],
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 108 {
            return Err(err("invalid HyperX QuadCast 2 S frame"));
        }
        h.write(&report::header()).map_err(err)?;
        for p in 0..6usize {
            let start = p * 20;
            let end = (start + 20).min(108);
            h.write(&report::color_packet(p, &colors[start..end]))
                .map_err(err)?;
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
