// SPDX-License-Identifier: GPL-2.0-or-later
// ThingM Blink USB HID module, OpenRGB commit bbc77470a7cba56af60989e54ba910d5ac66a3ac.
// Modified for freeRGB: brokered HID, Direct mode only, no serial/path reads.
pub mod devices_table;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};
pub const MODULE_ID: &str = "thingm-blink";
#[derive(Default)]
pub struct ThingmBlink {
    active: bool,
}
fn send(hid: &mut dyn Hid, p: &[u8; report::REPORT_LEN]) -> Result<(), ModuleError> {
    hid.send_feature(p).map(|_| ())
}
impl Module for ThingmBlink {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: MODULE_ID.into(),
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
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "ThingM blink(1) mk2".into()),
            kind: DeviceKind::LightingController,
            layout: LedLayout {
                width: 2,
                height: 1,
                leds: vec![
                    Led {
                        name: "LED A".into(),
                        pos: (0, 0),
                    },
                    Led {
                        name: "LED B".into(),
                        pos: (1, 0),
                    },
                ],
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![Zone {
                name: "blink(1) mk2".into(),
                leds: 2,
            }],
        })
    }
    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if colors.len() != 2 {
            return Err(ModuleError::new(
                ErrorCode::Protocol,
                "ThingM Blink requires two LED colors",
            ));
        }
        for (idx, c) in colors.iter().enumerate() {
            send(hid, &report::direct(idx as u8 + 1, c.r, c.g, c.b))?;
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
