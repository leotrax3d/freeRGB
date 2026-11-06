// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct RecordingWrite {
    writes: Vec<Vec<u8>>,
}
impl Hid for RecordingWrite {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn frame_is_byte_exact() {
    let mut m = crate::CorsairK65Mini::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let i = InterfaceInfo {
        vid: 0x1B1C,
        pid: 0x1BAF,
        interface: 1,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 64);
    assert_eq!(h.writes.len(), 3);
    assert_eq!(h.writes[0].len(), 1025);
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 64];
    colors[0] = Rgb { r: 1, g: 2, b: 3 };
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 4);
    let p = &h.writes[3];
    assert_eq!(p.len(), 1025);
    assert_eq!(
        &p[0..10],
        &[0, 0x08, 0x06, 0x01, 0x73, 0x01, 0x00, 0x00, 0x12, 0x00]
    );
    let off = 10 + 41 * 3;
    assert_eq!(&p[off..off + 3], &[1, 2, 3]);
    m.shutdown(None, &mut h).unwrap();
}
