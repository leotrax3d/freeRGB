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
pub const MODULE_ID: &str = "steelseries";
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}
/// One LED with its upstream zone/value mapping.
#[derive(Clone)]
struct LedSpec {
    name: &'static str,
    value: u8,
}
// upstream: RGBController_SteelSeriesRival.cpp rival_650_leds / rival_600_leds
const RIVAL_650_MOUSE_LEDS: [LedSpec; 6] = [
    LedSpec {
        name: "Left 1",
        value: 0x12,
    },
    LedSpec {
        name: "Left 2",
        value: 0x14,
    },
    LedSpec {
        name: "Left 3",
        value: 0x16,
    },
    LedSpec {
        name: "Right 1",
        value: 0x13,
    },
    LedSpec {
        name: "Right 2",
        value: 0x15,
    },
    LedSpec {
        name: "Right 3",
        value: 0x17,
    },
];
const RIVAL_600_MOUSE_LEDS: [LedSpec; 6] = [
    LedSpec {
        name: "Left top",
        value: 0x02,
    },
    LedSpec {
        name: "Left mid",
        value: 0x04,
    },
    LedSpec {
        name: "Left bottom",
        value: 0x06,
    },
    LedSpec {
        name: "Right top",
        value: 0x03,
    },
    LedSpec {
        name: "Right mid",
        value: 0x05,
    },
    LedSpec {
        name: "Right bottom",
        value: 0x07,
    },
];
/// upstream: RGBController_SteelSeriesRival.cpp SetupZones()
/// LED list per device type (values are the C++ led->value zone ids).
fn led_specs(proto: table::Proto) -> Vec<LedSpec> {
    match proto {
        // upstream: SetupZones base case ("Rival 100 Series only has one Zone")
        // and the "Rival 300 and 700 extend this" branch
        table::Proto::Rival100 => vec![LedSpec {
            name: "Logo",
            value: 0,
        }],
        table::Proto::Rival300 | table::Proto::Rival700 => vec![
            LedSpec {
                name: "Logo",
                value: 0,
            },
            LedSpec {
                name: "Scroll Wheel",
                value: 1,
            },
        ],
        table::Proto::Rival650 => {
            // upstream: leds[0].value = 0x11; wheel led value = 0x10
            [
                vec![LedSpec {
                    name: "Logo",
                    value: 0x11,
                }],
                vec![LedSpec {
                    name: "Scroll Wheel",
                    value: 0x10,
                }],
                RIVAL_650_MOUSE_LEDS.to_vec(),
            ]
            .concat()
        }
        table::Proto::Rival3 => {
            // upstream: SteelSeriesRival3Controller.h rival_3 led table
            vec![
                LedSpec {
                    name: "Front",
                    value: 0x01,
                },
                LedSpec {
                    name: "Middle",
                    value: 0x02,
                },
                LedSpec {
                    name: "Rear",
                    value: 0x03,
                },
                LedSpec {
                    name: "Logo",
                    value: 0x04,
                },
            ]
        }
        table::Proto::Rival5 => {
            // upstream: SteelSeriesRival5Controller.h rival_5 led table
            vec![
                LedSpec {
                    name: "Scroll Wheel",
                    value: 0,
                },
                LedSpec {
                    name: "Body Left 1",
                    value: 1,
                },
                LedSpec {
                    name: "Body Right 1",
                    value: 2,
                },
                LedSpec {
                    name: "Body Left 2",
                    value: 3,
                },
                LedSpec {
                    name: "Body Right 2",
                    value: 4,
                },
                LedSpec {
                    name: "Body Left 3",
                    value: 5,
                },
                LedSpec {
                    name: "Body Right 3",
                    value: 6,
                },
                LedSpec {
                    name: "Body Left 4",
                    value: 7,
                },
                LedSpec {
                    name: "Body Right 4",
                    value: 8,
                },
                LedSpec {
                    name: "Logo",
                    value: 9,
                },
            ]
        }
        table::Proto::Sensei => {
            // upstream: RGBController_SteelSeriesSensei.cpp SetupZones()
            // (led->value defaults to 0/1 = the zone ids used by SetColor)
            vec![
                LedSpec {
                    name: "Logo",
                    value: 0,
                },
                LedSpec {
                    name: "Scroll Wheel",
                    value: 1,
                },
            ]
        }
        table::Proto::Siberia350 => {
            // upstream: RGBController_SteelSeriesSiberia.cpp SetupZones()
            vec![LedSpec {
                name: "Headset LED",
                value: 0,
            }]
        }
        table::Proto::Arctis5 => {
            // upstream: RGBController_SteelSeriesArctis5.cpp SetupZones()
            // (zone ids 0/1 used by SetColor)
            vec![
                LedSpec {
                    name: "Left",
                    value: 0,
                },
                LedSpec {
                    name: "Right",
                    value: 1,
                },
            ]
        }
        table::Proto::ArctisNova3 => {
            // upstream: RGBController_SteelSeriesArctisNova3.cpp SetupZones()
            // (led order Right, Left; zone_id = led index)
            vec![
                LedSpec {
                    name: "Right",
                    value: 0,
                },
                LedSpec {
                    name: "Left",
                    value: 1,
                },
            ]
        }
        table::Proto::Aerox3 => {
            // upstream: SteelSeriesAerox3Controller.h aerox_3 led table
            // (zone_id = led index, packet bit mask 1 << zone_id)
            vec![
                LedSpec {
                    name: "Front",
                    value: 0,
                },
                LedSpec {
                    name: "Middle",
                    value: 1,
                },
                LedSpec {
                    name: "Rear",
                    value: 2,
                },
            ]
        }
        table::Proto::Aerox5 => {
            // upstream: SteelSeriesAerox5Controller.h aerox_5 led table
            vec![
                LedSpec {
                    name: "Front",
                    value: 0,
                },
                LedSpec {
                    name: "Middle",
                    value: 1,
                },
                LedSpec {
                    name: "Rear",
                    value: 2,
                },
                LedSpec {
                    name: "Reactive",
                    value: 3,
                },
            ]
        }
        table::Proto::AeroxWireless { aerox9, rival3, .. } => {
            // upstream: SteelSeriesAeroxWirelessController.h GetMouse tables
            let table: &[(&str, u8)] = if rival3 {
                &crate::report::AEROX_WIRELESS_RIVAL3_LEDS
            } else if aerox9 {
                &crate::report::AEROX_WIRELESS_A9_LEDS
            } else {
                &crate::report::AEROX_WIRELESS_A3_LEDS
            };
            table
                .iter()
                .map(|(name, value)| LedSpec {
                    name,
                    value: *value,
                })
                .collect()
        }
        table::Proto::Qck => {
            // upstream: RGBController_SteelSeriesQCKMat.cpp SetupZones()
            vec![
                LedSpec {
                    name: "Mat Bottom LED",
                    value: 0,
                },
                LedSpec {
                    name: "Mat Top LED",
                    value: 1,
                },
            ]
        }
        table::Proto::Rival600 => {
            // upstream: leds[0].value = 0x01; wheel led value = 0x00
            [
                vec![LedSpec {
                    name: "Logo",
                    value: 0x01,
                }],
                vec![LedSpec {
                    name: "Scroll Wheel",
                    value: 0x00,
                }],
                RIVAL_600_MOUSE_LEDS.to_vec(),
            ]
            .concat()
        }
    }
}
/// upstream: RGBController_SteelSeriesRival.cpp SetupZones()
/// Zone structure (name, led count) per device type.
fn zones(proto: table::Proto) -> Vec<(&'static str, usize)> {
    match proto {
        table::Proto::Rival100 => vec![("Logo", 1)],
        table::Proto::Rival300 | table::Proto::Rival700 => {
            vec![("Logo", 1), ("Scroll Wheel", 1)]
        }
        // upstream: RGBController_SteelSeriesRival3.cpp SetupZones() - one
        // single-LED zone per led_info entry
        table::Proto::Rival3 => vec![("Front", 1), ("Middle", 1), ("Rear", 1), ("Logo", 1)],
        // upstream: RGBController_SteelSeriesRival5.cpp SetupZones() - one
        // linear "Whole Mouse" zone with all 10 LEDs
        table::Proto::Rival5 => vec![("Whole Mouse", 10)],
        // upstream: RGBController_SteelSeriesSensei.cpp SetupZones()
        table::Proto::Sensei => vec![("Logo", 1), ("Scroll Wheel", 1)],
        // upstream: RGBController_SteelSeriesSiberia.cpp SetupZones()
        table::Proto::Siberia350 => vec![("Headset", 1)],
        // upstream: RGBController_SteelSeriesArctis5.cpp SetupZones()
        table::Proto::Arctis5 => vec![("Left", 1), ("Right", 1)],
        // upstream: RGBController_SteelSeriesQCKMat.cpp SetupZones()
        table::Proto::Qck => vec![("Mousemat", 2)],
        // upstream: RGBController_SteelSeriesArctisNova3.cpp SetupZones()
        table::Proto::ArctisNova3 => vec![("Nova 3", 2)],
        // upstream: SteelSeriesAerox3Controller.h aerox_3 (via
        // RGBController_SteelSeriesRival3 SetupZones) - one zone per LED
        table::Proto::Aerox3 => vec![("Front", 1), ("Middle", 1), ("Rear", 1)],
        // upstream: SteelSeriesAerox5Controller.h aerox_5 - one zone per LED
        table::Proto::Aerox5 => vec![("Front", 1), ("Middle", 1), ("Rear", 1), ("Reactive", 1)],
        // upstream: RGBController_SteelSeriesRival3.cpp SetupZones() - one
        // single-LED zone per led_info entry, named like the LED
        table::Proto::AeroxWireless { aerox9, rival3, .. } => {
            let table: &[(&str, u8)] = if rival3 {
                &crate::report::AEROX_WIRELESS_RIVAL3_LEDS
            } else if aerox9 {
                &crate::report::AEROX_WIRELESS_A9_LEDS
            } else {
                &crate::report::AEROX_WIRELESS_A3_LEDS
            };
            table.iter().map(|(name, _)| (*name, 1_usize)).collect()
        }
        table::Proto::Rival650 | table::Proto::Rival600 => {
            vec![("Logo", 1), ("Scroll Wheel", 1), ("Mouse", 6)]
        }
    }
}
fn kind(proto: table::Proto) -> DeviceKind {
    match proto {
        // upstream: RGBController_SteelSeriesSiberia.cpp type = DEVICE_TYPE_HEADSET
        table::Proto::Siberia350 | table::Proto::Arctis5 | table::Proto::ArctisNova3 => {
            DeviceKind::Headset
        }
        // upstream: RGBController_SteelSeriesQCKMat.cpp type = DEVICE_TYPE_MOUSEMAT
        table::Proto::Qck => DeviceKind::Mousemat,
        // upstream: RGBController_SteelSeriesRival*.cpp type = DEVICE_TYPE_MOUSE
        _ => DeviceKind::Mouse,
    }
}
pub struct SteelSeries {
    active: bool,
    proto: Option<table::Proto>,
    leds: Vec<LedSpec>,
    /// upstream: SteelSeriesRival5Controller.cpp sets member initialization
    /// `current_brightness = 0xFF` so the first SetBrightness call sends.
    rival5_brightness: u8,
    /// upstream: SteelSeriesAerox3Controller.cpp has an uninitialized
    /// current_brightness member; the first SetColor always sends the
    /// brightness packet, so we track None until the first frame.
    aerox3_brightness: Option<u8>,
    /// same guard pattern for the Aerox 5 wired path
    aerox5_brightness: Option<u8>,
    /// upstream: SteelSeriesAeroxWirelessController.cpp brightness guard
    /// (only 10 steps; Direct brightness 0x64 -> level 10)
    aerox_w_brightness: Option<u8>,
}
impl Default for SteelSeries {
    fn default() -> Self {
        SteelSeries {
            active: false,
            proto: None,
            leds: Vec::new(),
            rival5_brightness: 0xFF,
            aerox3_brightness: None,
            aerox5_brightness: None,
            aerox_w_brightness: None,
        }
    }
}
impl Module for SteelSeries {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "SteelSeriesController".into(),
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
        h: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let d = table::detector(i).ok_or_else(|| err("no SteelSeries HID detector"))?;
        let proto = table::proto_of_handler(d.handler)
            .ok_or_else(|| err("SteelSeries controller protocol not ported"))?;
        let leds = led_specs(proto);
        let zone_list = zones(proto);
        self.proto = Some(proto);
        self.leds = leds;
        self.active = true;
        if proto == table::Proto::Aerox3 || proto == table::Proto::Aerox5 {
            // upstream: SteelSeriesAerox3/5Controller.cpp SendInit() is called
            // in the controller constructor (same [00 2D] packet)
            h.send_feature(&report::aerox3_init()).map_err(err)?;
        }
        if let table::Proto::AeroxWireless { .. } = proto {
            // upstream: SteelSeriesAeroxWirelessController.cpp SendInit() in
            // the controller constructor (64-byte [00 2D] feature report)
            h.send_feature(&report::aerox_wireless_init())
                .map_err(err)?;
        }
        let count = self.leds.len();
        Ok(Capabilities {
            name: d.name.into(),
            kind: kind(proto),
            // Only Direct mode is exposed; upstream effect modes are out of scope.
            layout: LedLayout {
                width: count as u16,
                height: 1,
                leds: (0..count)
                    .map(|n| Led {
                        name: self.leds[n].name.into(),
                        pos: (n as u16, 0),
                    })
                    .collect(),
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: zone_list
                .into_iter()
                .map(|(name, leds)| Zone {
                    name: name.into(),
                    leds,
                })
                .collect(),
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
            return Err(err("SteelSeries module not initialized"));
        }
        if c.len() != self.leds.len() {
            return Err(err("invalid SteelSeries frame"));
        }
        let proto = self.proto.expect("active implies proto");
        if proto == table::Proto::Qck {
            // upstream: RGBController_SteelSeriesQCKMat.cpp DeviceUpdateLEDs()
            // -> SetColors expects both LEDs in one packet
            for pkt in report::qck_set_colors(c[0], c[1]) {
                if pkt.len() == 525 {
                    // upstream: SetColors uses hid_send_feature_report for
                    // the 525-byte buffer
                    h.send_feature(&pkt).map_err(err)?;
                } else {
                    h.write(&pkt).map_err(err)?;
                }
            }
            return Ok(());
        }
        if proto == table::Proto::Aerox3 {
            // upstream: RGBController_SteelSeriesRival3.cpp DeviceUpdateLEDs()
            // -> per-zone SetColor with brightness guard
            let level = report::MOUSE_BRIGHTNESS_MAX;
            if self.aerox3_brightness != Some(level) {
                self.aerox3_brightness = Some(level);
                h.write(&report::aerox3_set_brightness(level))
                    .map_err(err)?;
            }
            for (zone, color) in c.iter().enumerate() {
                h.write(&report::aerox3_set_color(zone as u8, *color))
                    .map_err(err)?;
            }
            return Ok(());
        }
        if let table::Proto::AeroxWireless {
            wireless,
            aerox9,
            rival3,
        } = proto
        {
            // upstream: RGBController_SteelSeriesRival3.cpp DeviceUpdateLEDs()
            // -> per-zone SetColor; brightness is applied right after the
            // first color packet when it changed (level = 0x64 / 10 = 10)
            let level = report::MOUSE_BRIGHTNESS_MAX / 10;
            for (zone, color) in c.iter().enumerate() {
                // upstream: SetColor receives leds[led].value as the zone id
                // (equal to the index for Aerox tables, 1 for the Rival 3
                // Wireless single "Scroll Wheel" LED)
                let zone_id = self.leds[zone].value;
                let pkt =
                    report::aerox_wireless_set_color(wireless, aerox9, rival3, zone_id, *color);
                h.write(&pkt).map_err(err)?;
                // upstream: WriteBuffer() does a hid_read_timeout readback
                // after every write in wireless mode
                if wireless {
                    h.read(64, 250).map_err(err)?;
                }
                if self.aerox_w_brightness != Some(level) {
                    self.aerox_w_brightness = Some(level);
                    let b = report::aerox_wireless_set_brightness(wireless, level);
                    h.write(&b).map_err(err)?;
                    if wireless {
                        h.read(64, 250).map_err(err)?;
                    }
                }
            }
            return Ok(());
        }
        if proto == table::Proto::Aerox5 {
            // upstream: RGBController_SteelSeriesRival3.cpp DeviceUpdateLEDs()
            // -> per-zone SetColor with brightness guard
            let level = report::MOUSE_BRIGHTNESS_MAX;
            if self.aerox5_brightness != Some(level) {
                self.aerox5_brightness = Some(level);
                h.write(&report::aerox5_set_brightness(level))
                    .map_err(err)?;
            }
            for (zone, color) in c.iter().enumerate() {
                h.write(&report::aerox5_set_color(zone as u8, *color))
                    .map_err(err)?;
            }
            return Ok(());
        }
        if proto == table::Proto::ArctisNova3 {
            // upstream: RGBController_SteelSeriesArctisNova3.cpp
            // DeviceUpdateSingleLED -> SetDirect per LED (zone_id = led index):
            // 521-byte effect packet, select 0x0A, apply
            for (zone, color) in c.iter().enumerate() {
                for pkt in report::set_arctis_nova3_direct(zone as u8, *color) {
                    h.write(&pkt).map_err(err)?;
                }
            }
            return Ok(());
        }
        if proto == table::Proto::Arctis5 {
            // upstream: RGBController_SteelSeriesArctis5.cpp
            // DeviceUpdateLEDs -> per zone SetColor (5x 37-byte packets)
            for (zone, color) in c.iter().enumerate() {
                for pkt in report::set_arctis5_color(zone as u8, *color) {
                    h.write(&pkt).map_err(err)?;
                }
            }
            return Ok(());
        }
        if proto == table::Proto::Siberia350 {
            // upstream: RGBController_SteelSeriesSiberia.cpp DeviceUpdateLEDs()
            // -> SetColor (4x 16-byte packets for the single LED)
            for pkt in report::set_siberia_color(c[0]) {
                h.write(&pkt).map_err(err)?;
            }
            return Ok(());
        }
        if proto == table::Proto::Rival5 {
            // upstream: RGBController_SteelSeriesRival5.cpp DeviceUpdateLEDs()
            // SetBrightness first (only on change; Direct brightness max 0x64),
            // then UpdateLEDs as one 64-byte feature report
            let level = report::MOUSE_BRIGHTNESS_MAX;
            if self.rival5_brightness != level {
                self.rival5_brightness = level;
                h.send_feature(&report::set_rival5_brightness(level))
                    .map_err(err)?;
            }
            h.send_feature(&report::set_rival5_direct(c)).map_err(err)?;
            return Ok(());
        }
        if proto == table::Proto::Rival3 {
            // upstream: RGBController_SteelSeriesRival3.cpp DeviceUpdateLEDs()
            // SetColor per zone (brightness max) first, then DeviceUpdateMode()
            // -> SetLightEffectAll(DIRECT - speed):
            for (led, color) in self.leds.iter().zip(c) {
                h.write(&report::set_mouse_color(
                    led.value,
                    *color,
                    report::MOUSE_BRIGHTNESS_MAX,
                ))
                .map_err(err)?;
            }
            h.write(&report::set_mouse_light_effect_all(
                report::MOUSE_DIRECT_VALUE,
            ))
            .map_err(err)?;
            return Ok(());
        }
        // upstream: RGBController_SteelSeriesRival.cpp DeviceUpdateMode()
        // Direct mode first applies the effect, then updates every LED.
        for pkt in report::set_light_effect_all(proto, report::EFFECT_DIRECT) {
            h.write(&pkt).map_err(err)?;
        }
        // upstream: RGBController_SteelSeriesRival.cpp DeviceUpdateLEDs()
        for (led, color) in self.leds.iter().zip(c) {
            for pkt in report::set_color(proto, led.value, *color) {
                if proto == table::Proto::Rival700 {
                    // upstream: SetRival700Color uses hid_send_feature_report
                    h.send_feature(&pkt).map_err(err)?;
                } else {
                    h.write(&pkt).map_err(err)?;
                }
            }
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
