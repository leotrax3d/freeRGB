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
    let p1 = report::packet1();
    assert_eq!(p1.len(), 64);
    assert_eq!(p1[1], 0x5A);
    assert_eq!(p1[2], 0x3A);
    assert_eq!(p1[3], 0x02);
    assert_eq!(p1[4], 0x06);
    assert_eq!(p1[5], 0x01);
    let p2 = report::packet2();
    assert_eq!(p2[3], 0x06);
    assert_eq!(p2[4], 0x04);
    assert_eq!(p2[6], 0x03);
    assert_eq!(p2[7], 0x01);
    assert_eq!(p2[9], 0x01);
    let p3 = report::packet3(Rgb {
        r: 10,
        g: 20,
        b: 30,
    });
    assert_eq!(p3[3], 0x09);
    assert_eq!(p3[4], 0x0A);
    assert_eq!(p3[6], 0x03);
    assert_eq!(p3[7], 0x01);
    assert_eq!(p3[8], 0x01);
    assert_eq!(p3[9], 0xFF);
    assert_eq!(p3[10], 30);
    assert_eq!(p3[11], 20);
    assert_eq!(p3[12], 10);
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x041E,
        pid: 0x3256,
        interface: 4,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::CreativeSoundBlasterXG6::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    assert!(h.writes.is_empty());
    m.frame(&[Rgb { r: 7, g: 8, b: 9 }], &mut h).unwrap();
    assert_eq!(h.writes.len(), 3);
    assert_eq!(h.writes[0], crate::report::packet1());
    assert_eq!(h.writes[1], crate::report::packet2());
    assert_eq!(
        h.writes[2],
        crate::report::packet3(Rgb { r: 7, g: 8, b: 9 })
    );
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 3);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.interface = 3;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::CreativeSoundBlasterXG6::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.width, 1);
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 2], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }], &mut h).is_ok());
}
