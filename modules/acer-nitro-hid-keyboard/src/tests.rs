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
    assert_eq!(
        report::direct(0, Rgb { r: 1, g: 2, b: 3 }),
        [0xA4, 0x21, 0x02, 100, 0x00, 0x01, 1, 2, 3, 0x01, 0x00]
    );
    assert_eq!(report::direct(3, Rgb { r: 4, g: 5, b: 6 })[9], 0x08);
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x0CF2,
        pid: 0x5130,
        interface: 0,
        usage_page: 0xFF5A,
        usage: 0x0001,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::AcerNitroHidKeyboard::default();
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
    assert_eq!(h.features.len(), 4);
    for (idx, c) in colors.iter().enumerate() {
        assert_eq!(h.features[idx], crate::report::direct(idx, *c));
    }
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 4);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.vid = 0;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::AcerNitroHidKeyboard::default();
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
