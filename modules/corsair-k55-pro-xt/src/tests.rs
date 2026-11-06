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
fn frame_is_byte_exact_count_and_sizes() {
    let mut m = crate::CorsairK55ProXt::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let i = InterfaceInfo {
        vid: 0x1B1C,
        pid: 0x1BA1,
        interface: 1,
        usage_page: 0xFF42,
        usage: 0,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 111);
    assert_eq!(h.writes.len(), 3);
    assert_eq!(h.writes[0].len(), 65);
    assert_eq!(&h.writes[0][0..6], &[0, 0x08, 0x01, 0x03, 0x00, 0x02]);
    assert_eq!(&h.writes[1][0..4], &[0, 0x08, 0x02, 0x5F]);
    assert_eq!(&h.writes[2][0..5], &[0, 0x08, 0x0D, 0x00, 0x01]);
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 111];
    colors[0] = Rgb { r: 1, g: 2, b: 3 };
    m.frame(&colors, &mut h).unwrap();
    // 3 init + 7 frame
    assert_eq!(h.writes.len(), 10);
    let first = &h.writes[3];
    let second = &h.writes[4];
    assert_eq!(first.len(), 65);
    assert_eq!(second.len(), 65);
    assert_eq!(&first[0..6], &[0, 0x08, 0x06, 0x00, 0x9B, 0x01]);
    assert_eq!(&second[0..4], &[0, 0x08, 0x07, 0x00]);
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn report_len_and_count() {
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 111];
    let packets = crate::report::frames(&colors);
    assert_eq!(packets.len(), 7);
    assert!(packets.iter().all(|p| p.len() == 65));
}
