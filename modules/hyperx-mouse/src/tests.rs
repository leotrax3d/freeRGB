// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: HyperXMouseControllerDetect.cpp registers 14 IP/PU/IPU rows with
    // distinct interface/usage-page qualifications (ANY = 255).
    assert_eq!(crate::devices_table::DETECTORS.len(), 14);
    assert_eq!(crate::devices_table::DEVICES.len(), 14);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x0951 || d.vid == 0x03f0));
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    let _ = d;
    InterfaceInfo {
        vid: 0x0951,
        pid,
        interface: 1,
        usage_page: 0xff01,
        usage: 0,
        product: None,
    }
}

#[test]
fn only_ported_devices_are_claimed() {
    let surge = crate::table::identify(&info_for(0x16d3));
    assert!(surge.supported, "Surge path is ported");
    for row in 0..crate::devices_table::DETECTORS.len() {
        let d = &crate::devices_table::DETECTORS[row];
        let dev = &crate::devices_table::DEVICES[row];
        let i = InterfaceInfo {
            vid: d.vid,
            pid: d.pid,
            interface: d.interface as i32,
            usage_page: d.usage_page,
            usage: d.usage,
            product: None,
        };
        let id = crate::table::identify(&i);
        assert_eq!(id.supported, dev.protocol == "surge");
    }
}

#[test]
fn wrong_interface_or_usage_page_is_not_supported() {
    // Surge: interface 1, usage page 0xFF01. A control interface must not match.
    let i = info_for(0x16d3);
    let id = crate::table::identify(&i);
    assert!(id.model.as_deref().is_some_and(|m| m.contains("Surge")));
    let mut i2 = i.clone();
    i2.interface = 0;
    assert!(!crate::table::identify(&i2).supported);
    let mut i3 = i;
    i3.usage_page = 0x0001;
    assert!(!crate::table::identify(&i3).supported);
}

struct RecordingFeature {
    features: Vec<Vec<u8>>,
}

impl RecordingFeature {
    fn new() -> Self {
        RecordingFeature {
            features: Vec::new(),
        }
    }
}
impl Hid for RecordingFeature {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

struct NoIo;
impl Hid for NoIo {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        panic!("no device I/O may happen for unsupported devices")
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        panic!("no device I/O may happen for unsupported devices")
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        panic!("no device I/O may happen for unsupported devices")
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        panic!("no device I/O may happen for unsupported devices")
    }
}

#[test]
fn unsupported_initialize_fails_without_io() {
    let mut m = crate::HyperX::default();
    let mut h = NoIo;
    // FPS Pro (0x16d7) uses the unported "fps" path
    assert!(m.initialize(&info_for(0x16d7), &mut h).is_err());
    // Haste (0x1727) uses the unported "haste" path
    assert!(m.initialize(&info_for(0x1727), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}

#[test]
fn surge_full_sequence_is_byte_exact() {
    let mut m = crate::HyperX::default();
    let mut h = RecordingFeature::new();
    let i = info_for(0x16d3);
    let caps = m.initialize(&i, &mut h).unwrap();
    // upstream: LED Strip 32 + Logo 1
    assert_eq!(caps.layout.leds.len(), 33);
    assert_eq!(caps.zones.len(), 2);
    assert_eq!(caps.zones[0].name, "LED Strip");
    assert_eq!(caps.zones[1].name, "Logo");
    assert_eq!(caps.kind, freergb_module_api::DeviceKind::Mouse);
    assert!(h.features.is_empty()); // initialize sends nothing
    let colors: Vec<Rgb> = (0..33)
        .map(|k| Rgb {
            r: k as u8,
            g: (2 * k) as u8,
            b: (3 * k) as u8,
        })
        .collect();
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
    let p = &h.features[0];
    // upstream: SendDirect 264-byte feature report [07 14 00 A0 ...]
    assert_eq!(p.len(), 264);
    assert_eq!(&p[0..4], &[0x07, 0x14, 0x00, 0xA0]);
    assert_eq!(p[0x08], 0);
    assert_eq!(p[0x08 + 31], 31);
    assert_eq!(p[0x28 + 1], 2);
    assert_eq!(p[0x48 + 2], 6);
    assert_eq!(&p[0x6C..0x6F], &[32, 64, 96]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 1); // no exit packets
}
