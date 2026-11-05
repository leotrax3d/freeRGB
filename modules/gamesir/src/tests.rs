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
    let mut m = crate::GameSir::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let i = InterfaceInfo {
        vid: 0x3537,
        pid: 0x100F,
        interface: 2,
        usage_page: 0xFF7A,
        usage: 0x0001,
        product: None,
    };
    let _ = m.initialize(&i, &mut h).unwrap();
    assert!(h.writes.is_empty()); // init none
    let col = Rgb {
        r: 10,
        g: 20,
        b: 30,
    };
    m.frame(&[col], &mut h).unwrap();
    assert_eq!(h.writes.len(), 1);
    let p = &h.writes[0];
    assert_eq!(p.len(), 65);
    assert_eq!(&p[0..6], &[0, 5, 8, 10, 1, 3]);
    assert_eq!(p[6], 10);
    assert_eq!(p[7], 20);
    assert_eq!(p[8], 30);
    let mut checksum = 0u16;
    for x in &p[1..=9] {
        checksum += *x as u16;
    }
    assert_eq!(p[10] as u16, checksum & 0xFF);
    m.shutdown(None, &mut h).unwrap();
}
