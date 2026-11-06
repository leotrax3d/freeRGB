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
pub const MODULE_ID: &str = "evision-keyboard";
#[derive(Default)]
pub struct EVision {
    v2: bool,
    active: bool,
    leds: usize,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for EVision {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "EVisionKeyboardController".into(),
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
            return Err(err(id
                .reason
                .unwrap_or_else(|| "unsupported EVision device".into())));
        }
        // upstream: RGBController_EVisionKeyboard.cpp SetupZones: one 126-LED
        // keyboard zone in a 6x23 matrix (12 blank cells).
        self.v2 = false;
        self.leds = 126;
        self.active = true;
        let mut fallback = table::PositionMap::new();
        let leds = (0..126u16)
            .map(|n| {
                let pos = table::v1_position(n)
                    .or_else(|| fallback.next_free())
                    .unwrap_or(((n % 23), (n / 23)));
                Led {
                    name: format!("Keyboard LED {n}"),
                    pos,
                }
            })
            .collect();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "EVision Keyboard".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 23,
                height: 6,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![Zone {
                name: "Keyboard".into(),
                leds: 126,
            }],
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.leds {
            return Err(err("invalid EVision frame"));
        }
        let bytes: Vec<u8> = c.iter().flat_map(|x| [x.r, x.g, x.b]).collect();
        // upstream: EVisionKeyboardController.cpp SetKeyboardColors: sequential
        // 0x36-byte color chunks with checksums; no Begin/End packets in the flow.
        for (off, chunk) in bytes.chunks(0x36).enumerate() {
            h.write(&report::v1_colors(chunk, off * 0x36))
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
