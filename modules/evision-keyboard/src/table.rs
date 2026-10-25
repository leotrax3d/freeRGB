// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::{DETECTORS, V1_MATRIX};
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub v2: bool,
    pub interface: u8,
    pub usage_page: u16,
}
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| {
        d.vid == i.vid
            && d.pid == i.pid
            && i32::from(d.interface) == i.interface
            && d.usage_page == i.usage_page
    });
    let d = match d {
        Some(d) => d,
        None => {
            return Identification {
                supported: false,
                model: None,
                kind: None,
                reason: Some("no EVision HID detector for this interface".into()),
            }
        }
    };
    if d.v2 {
        // The V2 protocol needs the keyvalue_map per-LED offsets (Write calls with
        // BEGIN/END CONFIGURE and read-back validation), not ported.
        return Identification {
            supported: false,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Keyboard),
            reason: Some(
                "EVision V2 detector is inventoried but its keyvalue-map protocol is not ported"
                    .into(),
            ),
        };
    }
    Identification {
        supported: true,
        model: Some(d.name.into()),
        kind: Some(DeviceKind::Keyboard),
        reason: None,
    }
}
/// (row, col) position per V1 LED index, from the upstream matrix map.
pub fn v1_position(index: u16) -> Option<(u16, u16)> {
    V1_MATRIX
        .iter()
        .position(|c| *c == Some(index))
        .map(|p| ((p % 23) as u16, (p / 23) as u16))
}
/// Deterministic fallback for LEDs that are not in the matrix map: the first
/// unused matrix cell (None cells), so every layout position stays unique.
pub struct PositionMap {
    free: Vec<(u16, u16)>,
}
impl PositionMap {
    pub fn new() -> Self {
        let free = (0..V1_MATRIX.len())
            .filter(|&i| V1_MATRIX[i].is_none())
            .map(|i| ((i % 23) as u16, (i / 23) as u16))
            .collect();
        Self { free }
    }
    pub fn next_free(&mut self) -> Option<(u16, u16)> {
        self.free.pop()
    }
}
impl Default for PositionMap {
    fn default() -> Self {
        Self::new()
    }
}
