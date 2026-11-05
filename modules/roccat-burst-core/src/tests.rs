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
    let mut m = crate::RoccatBurstCore::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x1E7D,
        pid: 0x2DE6,
        interface: 3,
        usage_page: 0xFF01,
        usage: 0x0001,
        product: None,
    };
    let _ = m.initialize(&i, &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
    assert_eq!(h.features[0], vec![0x0E, 0x06, 0x01, 0x01, 0x00, 0xFF]);
    m.frame(&[Rgb { r: 1, g: 2, b: 3 }], &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
    let p = &h.features[1];
    assert_eq!(p.len(), 11);
    assert_eq!(&p[0..6], &[0x0D, 0x0B, 1, 2, 3, 0]);
    m.shutdown(None, &mut h).unwrap();
}
