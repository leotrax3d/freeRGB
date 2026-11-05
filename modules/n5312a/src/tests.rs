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
    let mut m = crate::N5312a::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x4E53,
        pid: 0x5406,
        interface: 1,
        usage_page: 0xFF01,
        usage: 0x0001,
        product: None,
    };
    let _ = m.initialize(&i, &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
    assert_eq!(h.features[0][0], 0x07);
    assert_eq!(h.features[0][1], 0xA0);
    m.frame(&[Rgb { r: 7, g: 8, b: 9 }], &mut h).unwrap();
    assert_eq!(h.features.len(), 3);
    assert_eq!(h.features[1][0], 0x07);
    assert_eq!(h.features[1][1], 0x0B);
    assert_eq!(h.features[2][1], 0x0A);
}
