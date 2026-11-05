// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct Fake {
    writes: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        Ok(vec![0; 65])
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        Ok(vec![0; 65])
    }
}

#[test]
fn frame_is_byte_exact() {
    let mut m = crate::CoolerMasterMm712::default();
    let mut h = Fake { writes: Vec::new() };
    let i = InterfaceInfo {
        vid: 0x2516,
        pid: 0x0169,
        interface: 3,
        usage_page: 0xFF0A,
        usage: 0x0002,
        product: None,
    };
    let _ = m.initialize(&i, &mut h).unwrap();
    // 4 report packets were sent (3 normal + direct init)
    assert_eq!(h.writes.len(), 4);
    m.frame(&[Rgb { r: 1, g: 2, b: 3 }], &mut h).unwrap();
    assert_eq!(h.writes.len(), 5);
    let p = &h.writes[4];
    assert_eq!(p.len(), 65);
    assert_eq!(&p[0..7], &[0x00, 0x5A, 0x81, 0x03, 1, 2, 3]);
}
