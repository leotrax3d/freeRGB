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
    let mut m = crate::DarkProject::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let i = InterfaceInfo {
        vid: 0x195D,
        pid: 0x2061,
        interface: 2,
        usage_page: 0xFFC2,
        usage: 0x04,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 87);
    assert!(h.features.is_empty());
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 87];
    colors[0] = Rgb { r: 7, g: 8, b: 9 };
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
    let rg = &h.features[0];
    let ba = &h.features[1];
    assert_eq!(rg.len(), 256);
    assert_eq!(ba.len(), 256);
    assert_eq!(&rg[0..5], &[0x08, 0x07, 0x00, 0x00, 0x00]);
    assert_eq!(&ba[0..5], &[0x08, 0x07, 0x00, 0x01, 0x00]);
    // packet_map[0] == 5 -> RG red/blue offset 5, green offset 107
    assert_eq!(rg[10], 7);
    assert_eq!(rg[112], 8);
    assert_eq!(ba[10], 9);
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn report_len_and_count() {
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 87];
    let packets = crate::report::frames(&colors);
    assert_eq!(packets.len(), 2);
    assert!(packets.iter().all(|p| p.len() == 256));
}
