// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};

#[derive(Debug, Clone, PartialEq)]
pub struct Detector {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub interface: i32,
    pub usage_page: u16,
    /// Product string override: the Spatha X Dock shares PID 0x1979 with the
    /// 2.4GHz mouse; upstream switches on the product string. A None entry
    /// matches any product (including None).
    pub product: Option<&'static str>,
}

/// Upstream zone names from `RGBController_AsusAuraMouse.cpp`.
pub const ZONE_NAMES: [&str; 5] = ["Logo", "Scroll Wheel", "Underglow", "All", "Dock"];

fn matches(d: &Detector, i: &InterfaceInfo) -> bool {
    d.vid == i.vid
        && d.pid == i.pid
        && d.interface == i.interface
        && d.usage_page == i.usage_page
        && match (d.product, &i.product) {
            (Some(want), Some(actual)) => want.eq_ignore_ascii_case(actual),
            _ => true,
        }
}

pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| matches(d, i));
    Identification {
        supported: d.is_some(),
        model: d.map(|x| x.name.into()),
        kind: d.map(|_| DeviceKind::Mouse),
        reason: d.is_none().then(|| {
            "no ASUS Aura mouse device with a native Direct mode for this interface".into()
        }),
    }
}

pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| matches(d, i))
}
