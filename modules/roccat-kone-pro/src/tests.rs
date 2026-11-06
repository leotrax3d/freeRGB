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
    fn get_feature(&mut self, b: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        Ok(b.to_vec())
    }
}

#[test]
fn packets_are_byte_exact() {
    assert_eq!(
        crate::report::enter_direct(),
        vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]
    );
    let p = crate::report::direct(&[
        Rgb { r: 1, g: 2, b: 3 },
        Rgb { r: 4, g: 5, b: 6 },
    ]);
    assert_eq!(p[..11], [0x0d, 0x0b, 1, 2, 3, 4, 5, 6, 0, 0, 0]);
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    assert_eq!(h.features.len(), 1);
    assert_eq!(h.features[0], vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
    assert_eq!(h.features[1][..11], [0x0d, 0x0b, 1, 2, 3, 4, 5, 6, 0, 0, 0]);
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 2], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 1], &mut h).is_err());
    assert_eq!(caps.layout.leds.len(), 2);
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let mut i = info_for();
    i.interface = 2;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    let mut i = info_for();
    i.pid = 0x9999;
    assert!(!crate::table::identify(&i).supported);
}
