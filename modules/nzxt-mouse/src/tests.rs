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
        let mut v = vec![0u8; 64];
        v[0] = 0x43;
        v[1] = 0x86;
        Ok(v)
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
    let mut m = crate::NzxtMouse::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let i = InterfaceInfo {
        vid: 0x1E71,
        pid: 0x2100,
        interface: 0,
        usage_page: 0xFFCA,
        usage: 0x0001,
        product: None,
    };
    let _ = m.initialize(&i, &mut h).unwrap();
    let c = Rgb { r: 9, g: 8, b: 7 };
    m.frame(&[c, c, c, c, c, c], &mut h).unwrap();
    assert!(h.writes.len() >= 2);
    let p = &h.writes[1];
    assert_eq!(p.len(), 64);
    assert_eq!(p[0], 0x43);
    assert_eq!(p[1], 0xAE);
    assert_eq!(p[33], 9);
    assert_eq!(p[47], 7);
    m.shutdown(None, &mut h).unwrap();
}
