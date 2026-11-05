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
    let mut m = crate::PatriotViperMouse::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x0C45,
        pid: 0x7E18,
        interface: 2,
        usage_page: 0xFF18,
        usage: 0x01,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 7);
    assert_eq!(h.features.len(), 1);
    assert_eq!(h.features[0].len(), 64);
    m.frame(&[Rgb { r: 1, g: 2, b: 3 }; 7], &mut h).unwrap();
    assert_eq!(h.features.len(), 8);
    // first frame packet
    let p = &h.features[1];
    assert_eq!(p[0], 0x01);
    assert_eq!(p[1], 0x13);
    assert_eq!(p[2], 0x00);
    assert_eq!(p[3], 0xFF);
    assert_eq!(p[4], 1);
    assert_eq!(p[5], 2);
    assert_eq!(p[6], 3);
    m.shutdown(None, &mut h).unwrap();
}
