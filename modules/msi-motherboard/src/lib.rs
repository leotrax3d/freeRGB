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
    /// 112 vs 162 protocol variant, decided by the 0x52 probe
    variant: u8,
    /// upstream: MSIMotherboard162Controller.cpp `data` packet, kept as a
    /// stateful buffer across frames (untouched zones keep their values)
    data162: Vec<u8>,
    /// zone values in frame order (supported_zones from board config)
    zone_values_162: Vec<u8>,
    /// onboard LED count for the 162 path
    onboard_162: usize,
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
        if probe.len() >= 162 && probe.len() <= 163 {
            return self.initialize_162(i, id, hid, probe);
        }
        if probe.len() < report::PACKET_112 || probe.len() > report::PACKET_112 + 1 {
            return Err(err(format!(
                "MSI board answers with a {}-byte 0x52 report; only the 112/162-byte controller paths are ported",
                probe.len()
            )));
        }
        self.variant = 112;
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
        match self.variant {
            162 => self.frame_162(c, h)?,
            _ => {
                // upstream: DeviceUpdateLEDs() fills every zone/LED via
                // SetLedColor, then Update() sends exactly one 112-byte
                // feature report.
                h.send_feature(&report::direct_packet_112(c)).map_err(err)?;
            }
        }
        Ok(())
    }
    fn shutdown(&mut self, _: Option<Rgb>, _: &mut dyn Hid) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }
}

impl MsiMotherboard {
    /// upstream: MSIMotherboard162Controller.cpp constructor +
    /// RGBController_MSIMotherboard162.cpp SetupZones/DeviceUpdateLEDs
    fn initialize_162(
        &mut self,
        i: &InterfaceInfo,
        id: Identification,
        hid: &mut dyn Hid,
        probe: Vec<u8>,
    ) -> Result<Capabilities, ModuleError> {
        self.variant = 162;
        // upstream: constructor: ReadFwVersion, then ReadSettings into the
        // long-lived `data` packet (the probe read above already pulled the
        // 0x52 report)
        let mut versions = Vec::new();
        for (label, report_id) in [("APROM", 0xB0u8), ("LDROM", 0xB6u8)] {
            hid.write(&report::fw_request(report_id)).map_err(err)?;
            let response = hid.read(64, 0).map_err(err)?;
            versions.push(format!("{}: {}", label, report::fw_string(&response)));
        }
        self.fw = Some(versions.join(", "));
        hid.get_feature(&[0x52], report::PACKET_162).map_err(err)?;
        let mut data = vec![0u8; report::PACKET_162];
        data.copy_from_slice(&probe[..report::PACKET_162.min(probe.len())]);
        // upstream: report id 0x52 leads the packet (set by the probe request)
        data[0] = 0x52;
        // upstream: constructor sets save_data = 0 and the onboard master
        // colorFlags = USE_CUSTOM_COLOR | SYNC_SETTING_ONBOARD
        data[161] = 0;
        data[report::ZONE_OFFSETS_162[4] + 8] |= report::USE_CUSTOM_COLOR | report::SYNC_ONBOARD;
        let (onboard, zones) = {
            let pid = i.pid;
            let (onboard, z) = report::board_config_162(pid);
            (onboard, z.to_vec())
        };
        self.data162 = data;
        self.onboard_162 = onboard;
        self.zone_values_162 = {
            let mut z = zones;
            z.push(13);
            z
        };
        // LED table in supported_zones order; single-LED zones append no index
        let zone_name = |v: u8| match v {
            1 => "JRGB1",
            2 => "JRGB2",
            8 => "JRAINBOW1",
            9 => "JRAINBOW2",
            11 => "JCORSAIR",
            _ => "Onboard LEDs",
        };
        let mut leds: Vec<LedSpec> = Vec::new();
        for &v in &self.zone_values_162 {
            let name = zone_name(v);
            let count = if v == 13 { onboard } else { 1 };
            for k in 0..count {
                leds.push(LedSpec {
                    name: if count == 1 {
                        format!("{} LED", name)
                    } else {
                        format!("{} LED {}", name, k + 1)
                    },
                });
            }
        }
        self.leds = leds;
        self.active = true;
        let count = self.leds.len();
        let zones_out: Vec<Zone> = self
            .zone_values_162
            .iter()
            .map(|&v| Zone {
                name: zone_name(v).into(),
                leds: if v == 13 { onboard } else { 1 },
            })
            .collect();
        Ok(Capabilities {
            name: id.model.unwrap_or_else(|| "MSI Mystic Light".into()),
            kind: DeviceKind::LightingController,
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
            zones: zones_out,
        })
    }
    /// upstream: RGBController_MSIMotherboard162.cpp
    /// DeviceUpdateLEDs + UpdateLed (SetMode STATIC + SetZoneColor) + Update
    fn frame_162(&mut self, c: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if c.len() != self.leds.len() {
            return Err(err("invalid MSI frame"));
        }
        let mut cursor = 0usize;
        let zones = self.zone_values_162.clone();
        for &zone_value in &zones {
            let leds_in_zone = if zone_value == 13 {
                self.onboard_162
            } else {
                1
            };
            for led_idx in (0..leds_in_zone).rev() {
                let color = c[cursor + led_idx];
                // upstream: UpdateLed passes brightness 100 and speed 0 for
                // the Direct (= STATIC) mode value of the 162 bytes path
                report::set_mode_162(&mut self.data162, zone_value + led_idx as u8);
                report::set_zone_color_162(&mut self.data162, zone_value + led_idx as u8, color);
            }
            cursor += leds_in_zone;
        }
        self.data162[161] = 0; // save flag
        hid.send_feature(&self.data162).map_err(err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
