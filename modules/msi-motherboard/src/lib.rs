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

pub const MODULE_ID: &str = "msi-motherboard";

/// One LED with its upstream name.
struct LedSpec {
    name: String,
}

fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

#[derive(Default)]
pub struct MsiMotherboard {
    active: bool,
    leds: Vec<LedSpec>,
    fw: Option<String>,
}

impl Module for MsiMotherboard {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "MSIMotherboardController".into(),
            api: MODULE_API,
        }
    }
    fn identify(&self, i: &InterfaceInfo) -> Identification {
        table::identify(i)
    }
    fn initialize_with_zones(
        &mut self,
        i: &InterfaceInfo,
        _z: Option<&[Zone]>,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let id = table::identify(i);
        if !id.supported {
            return Err(err(id.reason.unwrap_or_else(|| "unsupported".into())));
        }
        // upstream: MSIMotherboardControllerDetect.cpp probes with report id
        // 0x52; only boards answering with the 112-byte packet use the
        // controller path ported here.
        let probe = hid.get_feature(&[0x52], report::PROBE_LEN).map_err(err)?;
        if probe.len() < report::PACKET_112 || probe.len() > report::PACKET_112 + 1 {
            return Err(err(format!(
                "MSI board answers with a {}-byte 0x52 report; only the 112-byte controller path is ported",
                probe.len()
            )));
        }
        // upstream: MSIMotherboard112Controller.cpp constructor calls
        // ReadFwVersion() (APROM 0xB0, LDROM 0xB6) and ReadSettings()
        // (112-byte 0x52 get report)
        let mut versions = Vec::new();
        for (label, report_id) in [("APROM", 0xB0u8), ("LDROM", 0xB6u8)] {
            hid.write(&report::fw_request(report_id)).map_err(err)?;
            let response = hid.read(64, 0).map_err(err)?;
            versions.push(format!("{}: {}", label, report::fw_string(&response)));
        }
        self.fw = Some(versions.join(", "));
        hid.get_feature(&[0x52], report::PACKET_112).map_err(err)?;
        // upstream: RGBController_MSIMotherboard112.cpp SetupZones() /
        // led_zones table: JRGB1, JRAINBOW1, JCORSAIR, Onboard LEDs (7 LEDs)
        let mut leds: Vec<LedSpec> = Vec::new();
        for (zone_name, led_count) in [
            ("JRGB1", 1usize),
            ("JRAINBOW1", 1usize),
            ("JCORSAIR", 1usize),
            ("Onboard LEDs", report::ONBOARD_ZONE_LEDS),
        ] {
            for led_idx in 0..led_count {
                // upstream: single-LED zones get "<zone> LED", multi-LED zones
                // append the 1-based index
                let name = if led_count == 1 {
                    format!("{} LED", zone_name)
                } else {
                    format!("{} LED {}", zone_name, led_idx + 1)
                };
                leds.push(LedSpec { name });
            }
        }
        self.leds = leds;
        self.active = true;
        let count = self.leds.len();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "MSI Mystic Light".into()),
            kind: DeviceKind::LightingController,
            // upstream: DEVICE_TYPE_MOTHERBOARD mapped to LightingController
            layout: LedLayout {
                width: count as u16,
                height: 1,
                leds: (0..count)
                    .map(|n| Led {
                        name: self.leds[n].name.clone(),
                        pos: (n as u16, 0),
                    })
                    .collect(),
            },
            firmware: self.fw.clone(),
            restore: Restore::LastFrame,
            zones: vec![
                Zone {
                    name: "JRGB1".into(),
                    leds: 1,
                },
                Zone {
                    name: "JRAINBOW1".into(),
                    leds: 1,
                },
                Zone {
                    name: "JCORSAIR".into(),
                    leds: 1,
                },
                Zone {
                    name: "Onboard LEDs".into(),
                    leds: report::ONBOARD_ZONE_LEDS,
                },
            ],
        })
    }
    fn initialize(
        &mut self,
        i: &InterfaceInfo,
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        self.initialize_with_zones(i, None, h)
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active {
            return Err(err("MSI module is not initialized"));
        }
        if c.len() != self.leds.len() {
            return Err(err("invalid MSI frame"));
        }
        // upstream: DeviceUpdateLEDs() fills every zone/LED via SetLedColor,
        // then Update() sends exactly one 112-byte feature report.
        h.send_feature(&report::direct_packet_112(c)).map_err(err)?;
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
