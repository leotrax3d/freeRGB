// SPDX-License-Identifier: GPL-2.0-or-later
pub mod keys;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};
pub const MODULE_ID: &str = "corsair-k55-pro-xt";
#[derive(Default)]
pub struct CorsairK55ProXt {
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
impl Module for CorsairK55ProXt {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "CorsairK55RGBPROXTController".into(),
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
        for packet in report::init_packets() {
            h.write(&packet).map_err(err)?;
        }
        self.active = true;
        let leds = (0..111usize)
            .map(|n| Led {
                name: format!("LED {}", n),
                pos: ((n % 24) as u16, (n / 24) as u16),
            })
            .collect::<Vec<_>>();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "Corsair K55 RGB PRO XT".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: 24,
                height: 6,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: vec![Zone {
                name: "Keyboard".into(),
                leds: 111,
            }],
        })
    }
    fn frame(&mut self, colors: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || colors.len() != 111 {
            return Err(err("invalid Corsair K55 RGB PRO XT frame"));
        }
        for packet in report::frames(colors) {
            h.write(&packet).map_err(err)?;
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
