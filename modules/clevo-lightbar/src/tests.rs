// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct RecordingFeature { features: Vec<Vec<u8>> }
impl Hid for RecordingFeature {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> { unreachable!() }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> { unreachable!() }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> { self.features.push(b.to_vec()); Ok(b.len()) }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> { unreachable!() }
}

#[test]
fn frame_is_byte_exact() {
    let mut m = crate::ClevoLightbar::default();
    let mut h = RecordingFeature { features: Vec::new() };
    let i = InterfaceInfo { vid: 0x048D, pid: 0x7001, interface: 0, usage_page: 0xFF03, usage: 0x02, product: None };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    assert!(h.features.is_empty());
    m.frame(&[Rgb { r: 10, g: 20, b: 30 }], &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
    assert_eq!(&h.features[0], &[0x14, 0x00, 0x01, 10, 20, 30, 0x00, 0x00]);
    assert_eq!(&h.features[1], &[0x08, 0x22, 0x01, 0x06, 0x64, 0x01, 0x00, 0x00]);
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn inventory_count() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}