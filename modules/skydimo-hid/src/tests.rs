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
fn frame_packet_count_and_crc() {
    let mut m = crate::SkydimoHid::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let i = InterfaceInfo {
        vid: 0x1A86,
        pid: 0xE316,
        interface: 0,
        usage_page: 0xFF00,
        usage: 0x0001,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 49);
    assert_eq!(h.writes.len(), 0);
    let c = vec![Rgb { r: 1, g: 2, b: 3 }; 49];
    m.frame(&c, &mut h).unwrap();
    // 3 data packets (60+60+27) + 1 end packet
    assert_eq!(h.writes.len(), 4);
    assert_eq!(h.writes[0].len(), 64);
    assert_eq!(h.writes[3][60], 0x1E);
    m.shutdown(None, &mut h).unwrap();
}
