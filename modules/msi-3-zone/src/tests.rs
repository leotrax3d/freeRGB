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
    let mut m = crate::Msi3Zone::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x1770,
        pid: 0xFF00,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 4);
    assert!(h.features.is_empty());
    m.frame(
        &[
            Rgb { r: 1, g: 2, b: 3 },
            Rgb { r: 4, g: 5, b: 6 },
            Rgb { r: 7, g: 8, b: 9 },
            Rgb {
                r: 10,
                g: 11,
                b: 12,
            },
        ],
        &mut h,
    )
    .unwrap();
    assert_eq!(h.features.len(), 7);
    assert_eq!(&h.features[0], &[1, 2, 64, 1, 1, 2, 3, 236]);
    assert_eq!(&h.features[1], &[1, 2, 64, 2, 4, 5, 6, 236]);
    assert_eq!(&h.features[2], &[1, 2, 64, 3, 7, 8, 9, 236]);
    assert_eq!(&h.features[3], &[1, 2, 64, 4, 10, 11, 12, 236]);
    assert_eq!(&h.features[6], &[1, 2, 64, 7, 10, 11, 12, 236]);
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn inventory_count() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}
