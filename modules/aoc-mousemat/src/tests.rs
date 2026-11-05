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
    let mut m = crate::AocMousemat::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x3938,
        pid: 0x1162,
        interface: 1,
        usage_page: 0xFF19,
        usage: 0xFF19,
        product: None,
    };
    let _ = m.initialize(&i, &mut h).unwrap();
    let c = Rgb { r: 1, g: 2, b: 3 };
    m.frame(&[c], &mut h).unwrap();
    let p = &h.features[0];
    assert_eq!(p.len(), 32);
    assert_eq!(p[0], 0x20);
    assert_eq!(p[1], 0x03);
    assert_eq!(p[2], 0x02);
    assert_eq!(p[6], 1);
    assert_eq!(p[7], 2);
    assert_eq!(p[8], 3);
    assert_eq!(p[9], 0xFF);
    assert_eq!(p[30], 0x32);
    m.shutdown(None, &mut h).unwrap();
}
