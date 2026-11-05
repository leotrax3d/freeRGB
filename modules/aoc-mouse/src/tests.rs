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
    let mut m = crate::AocMouse::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x3938,
        pid: 0x1179,
        interface: 1,
        usage_page: 0xFF19,
        usage: 0xFF19,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    assert!(h.features.is_empty());
    m.frame(
        &[Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }],
        &mut h,
    )
    .unwrap();
    assert_eq!(h.features.len(), 1);
    assert_eq!(h.features[0][0x33], 1);
    assert_eq!(h.features[0][0x36], 4);
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn inventory_count_none() {
    // no devices_table generator for AOC; skip
}
