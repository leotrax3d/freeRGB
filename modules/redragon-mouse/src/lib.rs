// SPDX-License-Identifier: GPL-2.0-or-later
// Redragon USB HID mouse module, ported from OpenRGB commit bbc77470a7cba56af60989e54ba910d5ac66a3ac.
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

pub const MODULE_ID: &str = "redragon-mouse";
#[derive(Default)]
pub struct RedragonMouse {
    supported: bool,
}
fn send(hid: &mut dyn Hid, p: &[u8; report::REPORT_LEN]) -> Result<(), ModuleError> {
    hid.send_feature(p).map(|_| ())
}
impl Module for RedragonMouse {
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
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(ModuleError::new(
                ErrorCode::Unsupported,
                id.reason.unwrap_or_else(|| "unsupported".into()),
            ));
        }
        send(hid, &report::profile_zero())?;
        send(hid, &report::apply())?;
        self.supported = true;
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Redragon mouse".into()),
            kind: DeviceKind::Mouse,
            layout: LedLayout {
                width: 1,
                height: 1,
                leds: vec![Led {
                    name: "Mouse".into(),
                    pos: (0, 0),
                }],
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![Zone {
                name: "Mouse".into(),
                leds: 1,
            }],
        })
    }
    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        let c = colors
            .first()
            .ok_or_else(|| ModuleError::new(ErrorCode::Protocol, "missing mouse color"))?;
        send(hid, &report::color(c.r, c.g, c.b))?;
        send(hid, &report::apply())
    }
    fn shutdown(&mut self, final_color: Option<Rgb>, hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if self.supported {
            if let Some(c) = final_color {
                send(hid, &report::color(c.r, c.g, c.b))?;
                send(hid, &report::apply())?;
            }
        }
        self.supported = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
