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
fn report_packets_exact() {
    use crate::report;
    let colors = [Rgb {
        r: 10,
        g: 20,
        b: 30,
    }; 104];
    let r = report::red_grn(&colors);
    let b = report::blue(&colors);
    assert_eq!(r.len(), 264);
    assert_eq!(b.len(), 264);
    assert_eq!(&r[0..4], &[0x07, 0x0E, 0x01, 0x01]);
    assert_eq!(&b[0..4], &[0x07, 0x0E, 0x01, 0x02]);
    // unchanged trailing bytes remain zero
    assert!(r[4..].iter().filter(|&x| *x != 0).count() > 0);
    // first logical LED (key 0x08) gets its color
    assert_eq!(r[0x08], 10);
    assert_eq!(r[0x08 + 128], 20);
    assert_eq!(b[0x08], 30);
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x264A,
        pid: 0x3006,
        interface: 1,
        usage_page: 0xFF01,
        usage: 0x0000,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown() {
    let mut m = crate::ThermaltakePoseidonZRgb::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 104);
    assert_eq!(caps.layout.width, 23);
    assert_eq!(caps.layout.height, 6);
    assert!(h.features.is_empty());
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 104];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
    assert_eq!(h.features[0], crate::report::red_grn(&colors));
    assert_eq!(h.features[1], crate::report::blue(&colors));
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
}

#[test]
fn wrong_device_not_supported() {
    let mut i = supported_interface();
    i.usage_page = 0x0000;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_checked() {
    let mut m = crate::ThermaltakePoseidonZRgb::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let _ = m.initialize(&supported_interface(), &mut h).unwrap();
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 103], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 104], &mut h).is_ok());
}
