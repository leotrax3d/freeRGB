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
    let mut m = crate::LegoDimensionsToypad::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let i = InterfaceInfo {
        vid: 0x0E6F,
        pid: 0x0241,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let _ = m.initialize(&i, &mut h).unwrap();
    let colors = [
        Rgb { r: 1, g: 2, b: 3 },
        Rgb { r: 4, g: 5, b: 6 },
        Rgb { r: 7, g: 8, b: 9 },
    ];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 4); // 1 init + 3 direct
    let p = &h.writes[1];
    assert_eq!(p[0], 0x00);
    assert_eq!(p[1], 0x55);
    assert_eq!(p[6], 1);
    assert_eq!(p[7], 2);
    assert_eq!(p[8], 3);
    // checksum: sum 1..9
    println!("P1={:?}", &p[..10]);
    let checksum: u8 = p[1..9].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    println!("SUM={}", checksum);
    println!("P9={}", p[9]);
    assert_eq!(p[9], checksum);
    assert_eq!(p.len(), 32);
    m.shutdown(None, &mut h).unwrap();
}
