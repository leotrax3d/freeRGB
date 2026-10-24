// SPDX-License-Identifier: GPL-2.0-or-later
//
// Device table types and the support policy of this port. Derived from
// OpenRGB RazerDevices.h / RazerController.cpp (commit in devices_table.rs).
// Modified for freeRGB: Rust types; explicit list of upstream code paths this
// port does not carry yet, so such devices are reported as unsupported
// instead of being driven incorrectly.
use crate::devices_table::{DETECTORS, DEVICES};
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};

pub const RAZER_VID: u16 = 0x1532;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matrix {
    None,
    Standard,
    Extended,
    Linear,
    ExtendedArgb,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    Single,
    Linear,
    Matrix,
}

#[derive(Debug)]
pub struct ZoneDef {
    pub name: &'static str,
    pub kind: ZoneType,
    pub rows: u32,
    pub cols: u32,
}

#[derive(Debug)]
pub struct Device {
    pub name: &'static str,
    pub pid: u16,
    pub kind: DeviceKind,
    pub matrix: Matrix,
    pub transaction_id: u8,
    pub led_id: u8,
    pub rows: u32,
    pub cols: u32,
    pub keyboard_layout: bool,
    pub zones: &'static [ZoneDef],
}

#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub function: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: Option<i32>,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
}

// PIDs with upstream special cases (RazerController.cpp constructor).
pub const LEVIATHAN_V2: u16 = 0x0532;
pub const LEVIATHAN_V2X: u16 = 0x054A;
pub const CHARGING_PAD_CHROMA: u16 = 0x0F26;
pub const CHROMA_MUG: u16 = 0x0F07;
pub const FIREFLY_HYPERFLUX: u16 = 0x0068;
pub const DEATHADDER_ESSENTIAL_V2: u16 = 0x0098;

impl Detector {
    /// Same rule as OpenRGB's REGISTER_HID_DETECTOR_{IPU,PU,I}: every given
    /// criterion must match.
    pub fn matches(&self, i: &InterfaceInfo) -> bool {
        self.vid == i.vid
            && self.pid == i.pid
            && self.interface.is_none_or(|v| v == i.interface)
            && self.usage_page.is_none_or(|v| v == i.usage_page)
            && self.usage.is_none_or(|v| v == i.usage)
    }
}

impl Device {
    pub fn led_count(&self) -> usize {
        self.zones.iter().map(|z| (z.rows * z.cols) as usize).sum()
    }

    /// Why this port cannot drive the device yet, if so.
    pub fn unsupported_reason(&self) -> Option<&'static str> {
        match self.matrix {
            Matrix::Standard | Matrix::Extended | Matrix::Linear => {}
            Matrix::ExtendedArgb => {
                return Some("addressable ARGB channel reports are not ported yet")
            }
            Matrix::Custom => return Some("per-device custom LED reports are not ported yet"),
            Matrix::None => return Some("device has no RGB matrix"),
        }
        if self.keyboard_layout {
            return Some("keyboard layout mapping (KeyboardLayoutManager) is not ported yet");
        }
        if self.rows == 0 || self.cols == 0 || self.led_count() != (self.rows * self.cols) as usize
        {
            return Some("zone table does not cover the device matrix");
        }
        // One row per report: 80 argument bytes minus the row header.
        let header = match self.matrix {
            Matrix::Extended => 5,
            Matrix::Standard => 4,
            _ => 2,
        };
        if header + self.cols as usize * 3 > 80 {
            return Some("row does not fit one report");
        }
        None
    }

    pub fn report_index(&self) -> u8 {
        if matches!(self.pid, LEVIATHAN_V2 | LEVIATHAN_V2X) {
            0x07
        } else {
            0x00
        }
    }

    pub fn needs_software_mode(&self) -> bool {
        matches!(
            self.pid,
            CHARGING_PAD_CHROMA | CHROMA_MUG | FIREFLY_HYPERFLUX
        )
    }

    pub fn max_brightness(&self) -> u8 {
        if self.pid == DEATHADDER_ESSENTIAL_V2 {
            100
        } else {
            255
        }
    }
}

pub fn device(pid: u16) -> Option<&'static Device> {
    // Upstream keeps the last match ("for all ... if pid == ... index = i").
    DEVICES.iter().rev().find(|d| d.pid == pid)
}

/// Local identification. Only interfaces that an upstream detector registers
/// for `DetectRazerControllers` are claimed.
pub fn identify(interface: &InterfaceInfo) -> (Option<&'static Device>, Identification) {
    let Some(detector) = DETECTORS.iter().find(|d| d.matches(interface)) else {
        return (
            None,
            Identification {
                supported: false,
                model: None,
                kind: None,
                reason: Some("no Razer detector for this interface".into()),
            },
        );
    };
    let device = device(interface.pid);
    let reason = if detector.function != "DetectRazerControllers" {
        Some(format!(
            "uses upstream {} which is not ported yet",
            detector.function
        ))
    } else {
        match device {
            None => Some("device table has no entry".into()),
            Some(d) => d.unsupported_reason().map(str::to_owned),
        }
    };
    (
        device.filter(|_| reason.is_none()),
        Identification {
            supported: reason.is_none(),
            model: Some(detector.name.to_owned()),
            kind: device.map(|d| d.kind),
            reason,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cobra() -> InterfaceInfo {
        InterfaceInfo {
            vid: RAZER_VID,
            pid: 0x00A3,
            interface: 0,
            usage_page: 0x0001,
            usage: 0x0002,
            product: None,
        }
    }

    #[test]
    fn cobra_is_identified_only_on_its_registered_interface() {
        let (device, id) = identify(&cobra());
        assert!(id.supported, "{id:?}");
        assert_eq!(device.unwrap().led_count(), 1);
        assert_eq!(device.unwrap().transaction_id, 0x1F);
        let mut other = cobra();
        other.interface = 1;
        assert!(!identify(&other).1.supported);
        other = cobra();
        other.usage = 0x06;
        assert!(!identify(&other).1.supported);
    }

    #[test]
    fn unported_code_paths_are_reported_not_guessed() {
        let mut unsupported = 0;
        for d in crate::devices_table::DETECTORS {
            let interface = InterfaceInfo {
                vid: d.vid,
                pid: d.pid,
                interface: d.interface.unwrap_or(0),
                usage_page: d.usage_page.unwrap_or(0),
                usage: d.usage.unwrap_or(0),
                product: None,
            };
            let (device, id) = identify(&interface);
            assert_eq!(id.supported, device.is_some());
            if !id.supported {
                assert!(id.reason.is_some());
                unsupported += 1;
            }
        }
        assert!(unsupported > 0);
    }
}
