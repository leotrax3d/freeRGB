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

/// upstream: MSIOptixController.cpp SetDirect
#[test]
fn report_direct_is_byte_exact() {
    use crate::report;
    let mut colors = [Rgb { r: 0, g: 0, b: 0 }; 12];
    colors[0] = Rgb { r: 1, g: 2, b: 3 };
    colors[11] = Rgb { r: 4, g: 5, b: 6 };
    let p = report::direct(&colors);
    assert_eq!(p.len(), 168);
    assert_eq!(p[0], 0x72);
    assert_eq!(p[1], 0x01);
    assert_eq!(p[6], 0x64);
    assert_eq!(p[10], 0x80);
    // hdr duplicated at 12
    assert_eq!(p[12], 0x01);
    assert_eq!(p[22], 0x00);
    // colors start at 95
    assert_eq!(&p[95..98], &[1, 2, 3]);
    let off = 95 + 11 * 3;
    assert_eq!(&p[off..off + 3], &[4, 5, 6]);
    // trailing zeros
    assert!(p[131..168].iter().all(|&b| b == 0));
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1462,
        pid: 0x3FA4,
        interface: 0,
        usage_page: 0xFF00,
        usage: 0x0001,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::MsiOptix::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 12);
    assert!(h.features.is_empty());
    let colors: Vec<Rgb> = (0..12)
        .map(|i| Rgb {
            r: i as u8,
            g: (i + 1) as u8,
            b: (i + 2) as u8,
        })
        .collect();
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
    let mut arr = [Rgb::default(); 12];
    arr.copy_from_slice(&colors);
    assert_eq!(h.features[0], crate::report::direct(&arr));
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.usage_page = 0x0000;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::MsiOptix::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.width, 12);
    for (idx, led) in caps.layout.leds.iter().enumerate() {
        assert!(led.pos.0 < caps.layout.width);
        assert_eq!(led.pos.0, idx as u16);
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 11], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 12], &mut h).is_ok());
}
