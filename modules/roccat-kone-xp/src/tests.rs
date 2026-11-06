// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn info_for() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: i32::from(d.interface),
        usage_page: d.usage_page,
        usage: d.usage,
        product: None,
    }
}

#[derive(Default)]
struct Fake {
    features: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> { unreachable!() }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> { unreachable!() }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> { self.features.push(b.to_vec()); Ok(b.len()) }
    fn get_feature(&mut self, b: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> { Ok(b.to_vec()) }
}

#[test]
fn enter_direct_and_direct_packets_are_byte_exact() {
    assert_eq!(crate::report::enter_direct(), vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]);
    let p = crate::report::direct(&[Rgb { r: 1, g: 2, b: 3 }; 20]);
    assert_eq!(p.len(), 122);
    assert_eq!(&p[0..6], &[0x0d, 0x7a, 1, 2, 3, 0xff]);
    assert_eq!(p[0], 0x0d);
    assert_eq!(p[1], 0x7a);
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 20);
    assert_eq!(h.features.len(), 1);
    assert_eq!(&h.features[0][..6], &[0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]);
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 20];
    colors[0] = Rgb { r: 9, g: 8, b: 7 };
    colors[19] = Rgb { r: 1, g: 2, b: 3 };
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
    assert_eq!(h.features[1].len(), 122);
    assert_eq!(&h.features[1][2..8], &[9, 8, 7, 0xff, 0xff, 0xff]);
    assert_eq!(h.features[1][116], 1);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    let sum: usize = caps.zones.iter().map(|z| z.leds).sum();
    assert_eq!(sum, 20);
    assert_eq!(caps.layout.leds.len(), 20);
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 20], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 19], &mut h).is_err());
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let mut i = info_for();
    i.pid = 0x9999;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
}
