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
pub const MODULE_ID: &str = "msi-monitor72";

#[derive(Default)]
pub struct M {
    leds: usize,
    reverse: bool,
    active: bool,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

impl Module for M {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "MSIMonitorController".into(),
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
        if !table::device(i) {
            return Err(err("unsupported MSI monitor"));
        }
        // upstream: MSIMonitorController.cpp GetLayoutVersion drains stale
        // replies, writes the 64-byte `01 B0` version query and reads a
        // 64-byte `[0x01 0x5A] <version>` response; an unanswered or
        // non-ACK reply means the legacy 0x71 controller, which the module
        // does not port.
        h.write(&report::version_query()).map_err(err)?;
        let r = h.read(report::QUERY_PACKET_SIZE, 500).map_err(err)?;
        let version = report::version_from_reply(&r)
            .ok_or_else(|| err("MSI monitor did not answer the layout version query"))?;
        // upstream: MSIMonitorControllerDetect.cpp MSIMonitorLayout picks
        // the 168-byte 0x72 table row; versions outside the table fall
        // through to the legacy controller.
        let (leds, reverse) = report::layout(version).ok_or_else(|| {
            err(format!(
                "MSI monitor layout version {version} is not on the 0x72 table"
            ))
        })?;
        self.leds = leds;
        self.reverse = reverse;
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "MSI Monitor".into()),
            kind: DeviceKind::Accessory,
            layout: LedLayout {
                width: leds as u16,
                height: 1,
                leds: (0..leds)
                    .map(|n| Led {
                        name: format!("LED {}", n + 1),
                        pos: (n as u16, 0),
                    })
                    .collect(),
            },
            firmware: Some(version.to_string()),
            restore: Restore::LastFrame,
            zones: vec![],
        })
    }

    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.leds {
            return Err(err("invalid MSI monitor frame"));
        }
        h.send_feature(&report::frame(c, self.reverse))
            .map_err(err)?;
        Ok(())
    }

    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests;
