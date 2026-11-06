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
        report::direct(Rgb {
            r: 11,
            g: 22,
            b: 33
        }),
        [0xA4, 0x65, 0x02, 100, 0x00, 0x01, 11, 22, 33, 0x01, 0x00]
    );
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
    let mut m = crate::AcerNitroHidLed::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    assert!(h.features.is_empty());
    m.frame(&[Rgb { r: 7, g: 8, b: 9 }], &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
    assert_eq!(
        h.features[0],
        crate::report::direct(Rgb { r: 7, g: 8, b: 9 })
    );
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.usage = 0x0002;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::AcerNitroHidLed::default();
    let mut h = RecordingFeature {
        features: Vec::new(),
    };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.width, 1);
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 2], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }], &mut h).is_ok());
}
