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

/// upstream: HPOmen30LController.cpp SendZoneUpdate direct-mode packet
#[test]
fn report_direct_is_byte_exact() {
    use crate::report;
    let p = report::direct(
        1,
        Rgb {
            r: 11,
            g: 22,
            b: 33,
        },
    );
    assert_eq!(p.len(), 58);
    assert_eq!(p[2], 0x12);
    assert_eq!(p[3], 0x04);
    assert_eq!(p[36], 0x01);
    assert_eq!(p[9], 11);
    assert_eq!(p[10], 22);
    assert_eq!(p[11], 33);
    assert_eq!(p[30], 0x64);
    let p7 = report::direct(7, Rgb { r: 0, g: 0, b: 0 });
    assert_eq!(p7[36], 0x07);
    assert_eq!(p7[8 + 6 * 4], 0x64);
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x103C,
        pid: 0x84FD,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::HpOmen30L::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 7);
    assert!(h.writes.is_empty());
    let colors = (0..7)
        .map(|i| Rgb {
            r: i as u8 * 10,
            g: i as u8 * 20,
            b: i as u8 * 30,
        })
        .collect::<Vec<_>>();
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 7);
    for (idx, c) in colors.iter().enumerate() {
        assert_eq!(h.writes[idx], crate::report::direct((idx as u8) + 1, *c));
    }
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 7);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.vid = 0x1234;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::HpOmen30L::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.width, 7);
    for (idx, led) in caps.layout.leds.iter().enumerate() {
        assert!(led.pos.0 < caps.layout.width);
        assert_eq!(led.pos.0, idx as u16);
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 6], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 7], &mut h).is_ok());
}
