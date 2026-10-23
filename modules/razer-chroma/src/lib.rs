// SPDX-License-Identifier: GPL-2.0-or-later
//
// freeRGB hardware module "razer-chroma": Razer HID devices driven by the
// common Razer feature-report protocol.
//
// Ported from OpenRGB (https://gitlab.com/CalcProgrammer1/OpenRGB), commit
// in devices_table.rs: RazerController.cpp (constructor, SetLEDs,
// razer_set_custom_frame, razer_set_mode_custom, razer_set_brightness,
// razer_get_firmware, razer_usb_send/receive), RGBController_Razer.cpp
// (SetupZones LED naming, Direct mode brightness), RazerControllerDetect.cpp
// (DetectRazerControllers interface selection).
// Copyright (C) 2021 Adam Honse (CalcProgrammer1) and OpenRGB contributors.
//
// Modified for freeRGB (see PROVENANCE.md): reports go through the freeRGB
// module host instead of hidapi; only Direct mode is offered; serial number,
// effect list and paired-receiver queries are not performed; matrix LEDs get
// row-qualified names so every LED name is unique.

pub mod devices_table;
pub mod guard;
pub mod report;
pub mod table;

use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo, Restore, Zone,
    MODULE_API,
};
use report::{Report, Target, REPORT_LEN};
use std::thread::sleep;
use std::time::Duration;
use table::{Device, Matrix};

pub const MODULE_ID: &str = "razer-chroma";

fn ms(n: u64) {
    sleep(Duration::from_millis(n));
}

#[derive(Default)]
pub struct RazerChroma {
    device: Option<&'static Device>,
    target: Option<Target>,
    guard: guard::DeviceGuard,
}

/// Upstream SetupZones: zone name, plus " LED n" when a zone has >1 LED.
/// Row-qualified ("LED r.c") for multi-row zones to keep names unique.
/// The kind of zone does not change the name (upstream ignores it too).
pub fn layout(device: &Device) -> (LedLayout, Vec<Zone>) {
    let mut leds = Vec::new();
    let mut zones = Vec::new();
    let mut y = 0u16;
    for zone in device.zones {
        for row in 0..zone.rows {
            for col in 0..zone.cols {
                let name = match (zone.rows * zone.cols, zone.rows > 1) {
                    (1, _) => zone.name.to_owned(),
                    (_, true) => format!("{} LED {}.{}", zone.name, row + 1, col + 1),
                    _ => format!("{} LED {}", zone.name, col + 1),
                };
                leds.push(Led {
                    name,
                    pos: (col as u16, y + row as u16),
                });
            }
        }
        y += zone.rows as u16;
        zones.push(Zone {
            name: zone.name.to_owned(),
            leds: (zone.rows * zone.cols) as usize,
        });
    }
    let width = device
        .zones
        .iter()
        .map(|z| z.cols as u16)
        .max()
        .unwrap_or(1);
    (
        LedLayout {
            width: width.max(1),
            height: y.max(1),
            leds,
        },
        zones,
    )
}

impl RazerChroma {
    fn send(&self, hid: &mut dyn Hid, report: Report) -> Result<(), ModuleError> {
        let report = report.sealed();
        let _lock = self.guard.lock();
        hid.send_feature(&report.0)?;
        Ok(())
    }

    fn receive(&self, hid: &mut dyn Hid, report_id: u8) -> Result<Report, ModuleError> {
        let bytes = hid.get_feature(&[report_id], REPORT_LEN)?;
        let mut report = [0u8; REPORT_LEN];
        let n = bytes.len().min(REPORT_LEN);
        report[..n].copy_from_slice(&bytes[..n]);
        Ok(Report(report))
    }

    /// razer_get_firmware: send, wait 5 ms, read the response.
    fn firmware(&self, hid: &mut dyn Hid, target: Target) -> Result<Option<String>, ModuleError> {
        let request = target.get_firmware();
        ms(2);
        self.send(hid, request.clone())?;
        ms(5);
        let response = self.receive(hid, target.report_id)?;
        Ok(response
            .answers(&request.sealed())
            .then(|| format!("v{}.{}", response.args()[0], response.args()[1])))
    }

    fn set_brightness(
        &self,
        hid: &mut dyn Hid,
        device: &Device,
        target: Target,
    ) -> Result<(), ModuleError> {
        let report = match device.matrix {
            Matrix::Standard | Matrix::Linear => {
                target.brightness_standard(device.max_brightness())
            }
            _ => target.brightness_extended(device.max_brightness()),
        };
        self.send(hid, report)
    }

    /// RazerController::SetLEDs: one report per matrix row with 1 ms gaps,
    /// then the custom-frame mode report that applies it.
    fn set_leds(&self, hid: &mut dyn Hid, colors: &[Rgb]) -> Result<(), ModuleError> {
        let (device, target) = (self.device.unwrap(), self.target.unwrap());
        let cols = device.cols as usize;
        for row in 0..device.rows as usize {
            let rgb: Vec<u8> = colors[row * cols..(row + 1) * cols]
                .iter()
                .flat_map(|c| [c.r, c.g, c.b])
                .collect();
            let stop = (cols - 1) as u8;
            ms(1);
            let report = match device.matrix {
                Matrix::Standard => target.frame_standard(row as u8, 0, stop, &rgb),
                Matrix::Extended => target.frame_extended(row as u8, 0, stop, &rgb),
                _ => target.frame_linear(0, stop, &rgb),
            };
            self.send(hid, report)?;
        }
        ms(1);
        let report = match device.matrix {
            Matrix::Extended => target.mode_custom_extended(),
            _ => target.mode_custom_standard(),
        };
        self.send(hid, report)
    }
}

impl Module for RazerChroma {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "razer-chroma".into(),
            api: MODULE_API,
        }
    }

    fn identify(&self, interface: &InterfaceInfo) -> Identification {
        table::identify(interface).1
    }

    fn initialize(
        &mut self,
        interface: &InterfaceInfo,
        hid: &mut dyn Hid,
    ) -> Result<Capabilities, ModuleError> {
        let (device, identification) = table::identify(interface);
        let device = device.ok_or_else(|| {
            ModuleError::new(
                ErrorCode::Unsupported,
                identification
                    .reason
                    .unwrap_or_else(|| "unsupported".into()),
            )
        })?;
        let target = Target {
            report_id: device.report_index(),
            transaction_id: device.transaction_id,
            led_id: device.led_id,
        };
        if device.needs_software_mode() {
            self.send(hid, target.device_mode(report::DEVICE_MODE_SOFTWARE, 0))?;
        }
        let firmware = self.firmware(hid, target)?;
        // Direct mode selected: brightness = max (RGBController_Razer).
        self.set_brightness(hid, device, target)?;
        self.device = Some(device);
        self.target = Some(target);
        let (layout, zones) = layout(device);
        Ok(Capabilities {
            name: device.name.into(),
            kind: device.kind,
            layout,
            firmware,
            restore: Restore::LastFrame,
            zones,
        })
    }

    fn frame(&mut self, colors: &[Rgb], hid: &mut dyn Hid) -> Result<(), ModuleError> {
        self.set_leds(hid, colors)
    }

    /// Upstream leaves the last custom frame on shutdown (no save to flash);
    /// the firmware's stored effect returns on the next power cycle.
    fn shutdown(&mut self, final_color: Option<Rgb>, hid: &mut dyn Hid) -> Result<(), ModuleError> {
        if let (Some(color), Some(device)) = (final_color, self.device) {
            let colors = vec![color; device.led_count()];
            self.set_leds(hid, &colors)?;
        }
        self.device = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
