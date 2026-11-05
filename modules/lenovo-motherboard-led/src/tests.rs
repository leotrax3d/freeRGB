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
fn report_direct_is_byte_exact() {
    use crate::report;
    let p = report::direct(
        0x12,
        Rgb {
            r: 10,
            g: 20,
            b: 30,
        },
    );
    assert_eq!(p.len(), 64);
    assert_eq!(p[0], 0xCC);
    assert_eq!(p[1], 0x12);
    assert_eq!(p[2], 0x01);
    assert_eq!(p[3], 0x01);
    assert_eq!(p[4], 0x04);
    assert_eq!(&p[5..8], &[10, 20, 30]);
    assert_eq!(p[33], 0x12);
    assert_eq!(&p[37..40], &[10, 20, 30]);
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x17EF,
        pid: 0xC955,
        interface: 0,
        usage_page: 0xFF89,
        usage: 0x00CC,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown() {
    let mut m = crate::LenovoMotherboardLed::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    assert!(h.writes.is_empty());
    m.frame(
        &[Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }],
        &mut h,
    )
    .unwrap();
    assert_eq!(h.writes.len(), 2);
    assert_eq!(
        h.writes[0],
        crate::report::direct(0x12, Rgb { r: 1, g: 2, b: 3 })
    );
    assert_eq!(
        h.writes[1],
        crate::report::direct(0x11, Rgb { r: 4, g: 5, b: 6 })
    );
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2);
}

#[test]
fn wrong_device_not_supported() {
    let mut i = supported_interface();
    i.usage_page = 0;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_checked() {
    let mut m = crate::LenovoMotherboardLed::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let _ = m.initialize(&supported_interface(), &mut h).unwrap();
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 1], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 2], &mut h).is_ok());
}
