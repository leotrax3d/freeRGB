// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct RecordingFeature {
    features: Vec<Vec<u8>>,
}
impl Hid for RecordingFeature {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn report_direct_is_byte_exact() {
    use crate::report;
    let colors = [
        Rgb { r: 1, g: 2, b: 3 },
        Rgb { r: 4, g: 5, b: 6 },
        Rgb { r: 7, g: 8, b: 9 },
        Rgb {
            r: 10,
            g: 11,
            b: 12,
        },
    ];
    let p = report::direct(&colors);
    assert_eq!(p.len(), 30);
    assert_eq!(p[0], 0x06);
    assert_eq!(p[1], 0x01);
    assert_eq!(p[2], 0x4C);
    assert_eq!(p[3], 0x06);
    assert_eq!(p[4], 0x14);
    assert_eq!(p[5], 0x01);
    assert_eq!(p[6], 0xFF);
    assert_eq!(&p[7..10], &[1, 2, 3]);
    assert_eq!(p[10], 0x02);
    assert_eq!(&p[12..15], &[4, 5, 6]);
    assert_eq!(p[15], 0x03);
    assert_eq!(&p[17..20], &[7, 8, 9]);
    assert_eq!(p[20], 0x04);
    assert_eq!(&p[22..25], &[10, 11, 12]);
    assert!(p[25..30].iter().all(|&b| b == 0));
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1E7D,
        pid: 0x2CA6,
        interface: 0,
        usage_page: 0x0001,
        usage: 0x0002,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::RoccatBurstProAir::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 4);
    assert!(h.features.is_empty());
    let colors = [
        Rgb { r: 1, g: 2, b: 3 },
        Rgb { r: 4, g: 5, b: 6 },
        Rgb { r: 7, g: 8, b: 9 },
        Rgb {
            r: 10,
            g: 11,
            b: 12,
        },
    ];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
    let mut arr = [Rgb::default(); 4];
    arr.copy_from_slice(&colors);
    assert_eq!(h.features[0], crate::report::direct(&arr));
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.pid = 0x0000;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::RoccatBurstProAir::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.width, 4);
    for (idx, led) in caps.layout.leds.iter().enumerate() {
        assert!(led.pos.0 < caps.layout.width);
        assert_eq!(led.pos.0, idx as u16);
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 3], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 4], &mut h).is_ok());
}
