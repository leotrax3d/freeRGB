// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct RecordingFeature {
    features: Vec<Vec<u8>>,
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

#[test]
fn frame_is_byte_exact() {
    let mut m = crate::Tecknet::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x04D9,
        pid: 0xFC05,
        interface: 0,
        usage_page: 0xFFA0,
        usage: 0x01,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    assert!(h.features.is_empty());
    m.frame(&[Rgb { r: 1, g: 2, b: 3 }], &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
    let p = &h.features[0];
    assert_eq!(p.len(), 16);
    assert_eq!(p[0], 0x02);
    assert_eq!(p[1], 0x04);
    assert_eq!(p[2], 254);
    assert_eq!(p[3], 253);
    assert_eq!(p[4], 252);
    assert_eq!(p[5], 0x03);
    assert_eq!(p[6], 0x00);
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn inventory_count() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}
