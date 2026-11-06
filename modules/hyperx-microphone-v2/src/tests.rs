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
fn report_packets_exact() {
    use crate::report;
    let h = report::header();
    assert_eq!(h.len(), 64);
    assert_eq!(h[0], 0x44);
    assert_eq!(h[1], 0x01);
    assert_eq!(h[2], 0x06);
    assert!(h[3..].iter().all(|&b| b == 0));
    let colors = [Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }];
    let p = report::color_packet(0, &colors);
    assert_eq!(p[0], 0x44);
    assert_eq!(p[1], 0x02);
    assert_eq!(p[2], 0x00);
    assert_eq!(p[3], 0x00);
    assert_eq!(&p[4..10], &[1, 2, 3, 4, 5, 6]);
    let p5 = report::color_packet(5, &[]);
    assert_eq!(p5[2], 0x05);
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x03F0,
        pid: 0x02B5,
        interface: 1,
        usage_page: 0xFF13,
        usage: 0xFF00,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown() {
    let mut m = crate::HyperXMicrophoneV2::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 108);
    assert_eq!(caps.layout.width, 12);
    assert_eq!(caps.layout.height, 9);
    assert!(h.writes.is_empty());
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 108];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 7);
    assert_eq!(h.writes[0], crate::report::header());
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 7);
}

#[test]
fn wrong_device_not_supported() {
    let mut i = supported_interface();
    i.interface = 0;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_checked() {
    let mut m = crate::HyperXMicrophoneV2::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let _ = m.initialize(&supported_interface(), &mut h).unwrap();
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 107], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 108], &mut h).is_ok());
}
