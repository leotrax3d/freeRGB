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
pub const MODULE_ID: &str = "asus-legacy-usb";
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
            family: "AsusLegacyUSBController".into(),
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
        table::device(i).ok_or_else(|| err("unsupported ASUS legacy device"))?;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "ASUS legacy USB".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 1,
                height: 1,
                leds: (0..1)
                    .map(|n| Led {
                        name: format!("LED {n}"),
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
        if !self.active || c.len() != 1 {
            return Err(err("invalid ASUS legacy frame"));
        }
        for packet in report::frames(c[0]) {
            h.send_feature(&packet).map_err(err)?;
        }
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn inventoried_devices_are_not_claimed_before_protocol_port() {
        let i = crate::InterfaceInfo {
            vid: 0x195d,
            pid: 0x2047,
            interface: 1,
            usage_page: 0xff01,
            usage: 1,
            product: None,
        };
        let result = crate::table::identify(&i);
        assert!(!result.supported);
        assert!(result.reason.unwrap().contains("protocol is not ported"));
    }
}
