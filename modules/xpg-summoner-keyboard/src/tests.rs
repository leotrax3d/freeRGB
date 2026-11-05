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
fn report_packets_are_byte_exact() {
    use crate::report;
    let init = report::init_report();
    assert_eq!(init.len(), 265);
    assert_eq!(&init[0..4], &[0x07, 0xEA, 0x00, 0x00]);
    let data = [0u8; 256];
    let p0 = report::chunk(0, &data);
    assert_eq!(p0.len(), 265);
    assert_eq!(&p0[0..6], &[0x07, 0xA3, 0x08, 0x00, 0x00, 0x00]);
    let data2 = [0u8; 10];
    let p1 = report::chunk(1, &data2);
    assert_eq!(p1[4], 1);
    assert_eq!(p1[6], 0);
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x125F,
        pid: 0x9418,
        interface: 2,
        usage_page: 0xFF01,
        usage: 0x0001,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown() {
    let mut m = crate::XpgSummonerKeyboard::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 104);
    assert_eq!(h.writes.len(), 1);
    assert_eq!(h.writes[0].len(), 265);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 104];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 3);
    assert_eq!(h.writes[1].len(), 265);
    assert_eq!(h.writes[1][4], 0);
    assert_eq!(h.writes[2][4], 1);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 3);
}

#[test]
fn wrong_device_not_supported() {
    let mut i = supported_interface();
    i.pid = 0;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_checked() {
    let mut m = crate::XpgSummonerKeyboard::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let _ = m.initialize(&supported_interface(), &mut h).unwrap();
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 103], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 104], &mut h).is_ok());
}
