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
fn keyboard_frame_is_byte_exact() {
    let mut m = crate::Ionico::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x048D,
        pid: 0xCE00,
        interface: 0,
        usage_page: 0xFF12,
        usage: 0x0001,
        product: None,
    };
    let _ = m.initialize(&i, &mut h).unwrap();
    assert_eq!(h.features.len(), 1); // SetMode Direct
    let colors = [
        Rgb { r: 1, g: 2, b: 3 },
        Rgb { r: 4, g: 5, b: 6 },
        Rgb { r: 7, g: 8, b: 9 },
        Rgb {
            r: 10,
            g: 11,
            b: 12,
        },
    ];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 5);
    let first = &h.features[1];
    assert_eq!(&first[0..7], &[0x00, 0x14, 0x00, 0x01, 1, 2, 3]);
    m.shutdown(None, &mut h).unwrap();
}
