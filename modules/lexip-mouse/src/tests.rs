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
    let mut m = crate::LexipMouse::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let i = InterfaceInfo {
        vid: 0x04D8,
        pid: 0xFD0A,
        interface: 0,
        usage_page: 2,
        usage: 0x0001,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    let col = Rgb {
        r: 10,
        g: 20,
        b: 30,
    };
    m.frame(&[col], &mut h).unwrap();
    assert_eq!(h.writes.len(), 1);
    let p = &h.writes[0];
    assert_eq!(p.len(), 64);
    assert_eq!(p[1], 0x24);
    assert_eq!(p[2], 0x01);
    assert_eq!(p[3], 10);
    assert_eq!(p[4], 20);
    assert_eq!(p[5], 30);
    assert_eq!(p[7], 0x64);
    assert_eq!(p[8], 0x80);
    m.shutdown(None, &mut h).unwrap();
}
