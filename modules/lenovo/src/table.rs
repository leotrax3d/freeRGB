// SPDX-License-Identifier: GPL-2.0-or-later
use crate::devices_table::DETECTORS;
use freergb_module_api::{DeviceKind, Identification, InterfaceInfo};
/// (pid, protocol) pairs with a fully ported Direct-mode path.
pub const SUPPORTED: &[(u16, &str)] = &[
    (0xC963, "four-zone"),
    (0xC985, "four-zone"),
    (0xC984, "four-zone"),
    (0xC975, "four-zone"),
    (0xC973, "four-zone"),
    (0xC965, "four-zone"),
    (0xC955, "four-zone"),
    (0x60D5, "k500"),
    (0x60E4, "m300"),
    (0xC935, "legacy"),
    (0xC957, "legacy"),
    (0xC967, "legacy"),
    (0xC977, "gen7-8"),
];
#[derive(Debug)]
pub struct Detector {
    pub name: &'static str,
    pub handler: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub protocol: &'static str,
    pub leds: usize,
}
#[derive(Debug)]
pub struct Device {
    pub vid: u16,
    pub pid: u16,
    pub protocol: &'static str,
    pub leds: usize,
}
/// Protocols/pairs whose Direct-mode path is fully ported and audited:
/// four-zone (33-byte feature), k500 (Customize feature + RGB write), m300
/// (Static write), legacy ANSI framing (Y740/Y760S/Y760 feature groups) and the
/// byte-exact Gen7 keyboard 0xC977.
///
/// Not supported (audit): k510 0x619A (frame sends no packets), Gen10 0xC195/0xC197
/// (wrong packet header + missing D0 re-assert + 0xC197 missing logo LED 0xDD),
/// Gen7 laptops with logo/vents/neon zones (0xC978/0xC987/0xC988/0xC997/0xC998:
/// zone-ID high bytes zeroed), Y750 0xC956 (table LED count 145 makes the
/// additional LEDs unreachable).
pub fn identify(i: &InterfaceInfo) -> Identification {
    let d = DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid);
    match d {
        Some(d) if SUPPORTED.contains(&(d.pid, d.protocol)) => Identification {
            supported: true,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Accessory),
            reason: None,
        },
        Some(d) => Identification {
            supported: false,
            model: Some(d.name.into()),
            kind: Some(DeviceKind::Accessory),
            reason: Some(format!(
                "Lenovo detector {} is inventoried but its {} protocol path is not fully ported (audit)",
                d.name, d.protocol
            )),
        },
        None => Identification {
            supported: false,
            model: None,
            kind: None,
            reason: Some("no Lenovo HID detector for this VID/PID".into()),
        },
    }
}
pub fn device(i: &InterfaceInfo) -> Option<&'static Detector> {
    DETECTORS.iter().find(|d| d.vid == i.vid && d.pid == i.pid)
}
