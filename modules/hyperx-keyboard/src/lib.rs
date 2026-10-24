// SPDX-License-Identifier: GPL-2.0-or-later
pub mod devices_table;
pub mod layouts;
pub mod report;
pub mod table;
use freergb_core::{Led, LedLayout, Rgb};
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{
    Capabilities, DeviceKind, ErrorCode, Identification, InterfaceInfo, ModuleError, ModuleInfo,
    Restore, Zone, MODULE_API,
};
pub const MODULE_ID: &str = "hyperx-keyboard";
#[derive(Default)]
pub struct HyperX {
    kind: &'static str,
    leds: usize,
    active: bool,
}
fn err(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::new(ErrorCode::Protocol, e.to_string())
}

/// LED list for a layout: matrix zones place their LEDs via the matrix map
/// (cell value = LED index), linear zones follow the LED names in order.
fn leds_from_layout(l: &layouts::Layout) -> Vec<Led> {
    let total: usize = l.zone_sizes.iter().sum();
    // reverse map: LED index -> (row, col) for matrix zones (flat row-major matrix)
    let mut index_pos: Vec<(u16, u16)> = Vec::new();
    let mut start = 0usize;
    for (idx, &size) in l.zone_sizes.iter().enumerate() {
        if l.zone_types.get(idx).copied() == Some("MATRIX") {
            for (i, cell) in l.matrix.iter().enumerate() {
                let Some(v) = cell else { continue };
                let v = usize::from(*v);
                if v >= start && v < start + size && index_pos.len() <= v {
                    index_pos.resize(v + 1, (0, 0));
                    index_pos[v] = (
                        (i % usize::from(l.matrix_width)) as u16,
                        (i / usize::from(l.matrix_width)) as u16,
                    );
                    let _ = 0; // removed debug hook
                }
            }
        }
        start += size;
    }
    let mut leds = Vec::with_capacity(total);
    start = 0usize;
    for (idx, &size) in l.zone_sizes.iter().enumerate() {
        let zone_name = l.zone_names.get(idx).copied().unwrap_or("Zone");
        for k in 0..size {
            let index = start + k;
            let pos = index_pos
                .get(index)
                .copied()
                .unwrap_or((u16::try_from(k).unwrap_or(0), idx as u16));
            let name = l.led_names.get(index).copied().unwrap_or(zone_name);
            leds.push(Led {
                name: format!("{name} LED {}", k + 1),
                pos,
            });
        }
        start += size;
    }
    leds
}

impl Module for HyperX {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: MODULE_ID.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            family: "HyperXKeyboardController".into(),
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
        let d = table::device(i).ok_or_else(|| err("unsupported HyperX device"))?;
        if !table::identify(i).supported {
            return Err(err(
                "HyperX protocol path for this device is not fully ported (audit)",
            ));
        }
        self.kind = d.kind;
        let Some(l) = layouts::layout(self.kind) else {
            return Err(err("no upstream layout for this HyperX device"));
        };
        let leds = leds_from_layout(&l);
        self.leds = leds.len();
        self.active = true;
        Ok(Capabilities {
            name: table::identify(i)
                .model
                .unwrap_or_else(|| "HyperX Keyboard".into()),
            kind: DeviceKind::Keyboard,
            layout: LedLayout {
                width: l.matrix_width,
                height: l.matrix_height,
                leds,
            },
            firmware: None,
            restore: Restore::LastFrame,
            zones: l
                .zone_names
                .iter()
                .zip(l.zone_sizes.iter())
                .map(|(name, &size)| Zone {
                    name: name.to_string(),
                    leds: size,
                })
                .collect(),
        })
    }
    fn frame(&mut self, c: &[Rgb], h: &mut dyn Hid) -> Result<(), ModuleError> {
        if !self.active || c.len() != self.leds {
            return Err(err("invalid HyperX frame"));
        }
        // upstream: Alloy FPS/Elite send three 264-byte channel feature reports
        // (R/G/B planes, channel 1/2/3) plus a fourth packet for the media and
        // indicator LEDs on the Elite; all via hid_send_feature_report.
        match self.kind {
            "elite" => {
                for ch in [0x01_u8, 0x02, 0x03] {
                    h.send_feature(&report::channel(c, ch, ch)).map_err(err)?;
                }
                h.send_feature(&report::extended(c)).map_err(err)?;
            }
            "fps" => {
                for ch in [0x01_u8, 0x02, 0x03] {
                    h.send_feature(&report::channel(c, ch, ch)).map_err(err)?;
                }
            }
            _ => return Err(err("uncovered HyperX frame path")),
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
