// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct RecordingWrite { writes: Vec<Vec<u8>> }
impl Hid for RecordingWrite {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec()); Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> { unreachable!() }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> { unreachable!() }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> { unreachable!() }
}

#[test]
fn init_and_color_are_byte_exact() {
    let mut m = crate::DreamCheeky::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let i = InterfaceInfo { vid: 0x1D34, pid: 0x0004, interface: 0, usage_page: 0, usage: 0, product: None };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    assert_eq!(h.writes.len(), 4);
    assert_eq!(h.writes[0], vec![0x00, 0x1F, 0x02, 0x00, 0x5F, 0x00, 0x00, 0x1F, 0x03]);
    m.frame(&[Rgb { r: 255, g: 128, b: 0 }], &mut h).unwrap();
    assert_eq!(h.writes.len(), 5);
    let p = &h.writes[4];
    assert_eq!(p[0], 0x00);
    assert_eq!(p[1], (255 >> 2) + 1);
    assert_eq!(p[2], 128 >> 2);
    assert_eq!(p[3], 0);
    assert_eq!(p[8], 0x05);
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn inventory_count() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}