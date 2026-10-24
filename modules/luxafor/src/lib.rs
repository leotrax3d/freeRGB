// SPDX-License-Identifier: GPL-2.0-or-later
// Luxafor Flag USB HID module, OpenRGB commit bbc77470a7cba56af60989e54ba910d5ac66a3ac.
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
pub const MODULE_ID: &str = "luxafor";
#[derive(Default)]
pub struct Luxafor {
    active: bool,
}
fn send(hid: &mut dyn Hid, packet: &[u8; report::REPORT_LEN]) -> Result<(), ModuleError> {
    hid.write(packet).map(|_| ())
}
impl Module for Luxafor {
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
        let leds = (0..6)
            .map(|n| Led {
                name: if n < 3 {
                    "Flag LED".into()
                } else {
                    "Rear LED".into()
                },
                pos: ((n % 3) as u16, (n / 3) as u16),
            })
            .collect();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Luxafor Flag".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: 3,
                height: 2,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![
                Zone {
                    name: "Flag".into(),
                    leds: 3,
                },
                Zone {
                    name: "Rear".into(),
                    leds: 3,
                },
            ],
        })
    }
    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if colors.len() != 6 {
            return Err(ModuleError::new(
                ErrorCode::Protocol,
                "Luxafor requires six LED colors",
            ));
        }
        for (idx, c) in colors.iter().enumerate() {
            send(
                hid,
                &report::direct(report::LED_FIRST + idx as u8, c.r, c.g, c.b),
            )?;
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
