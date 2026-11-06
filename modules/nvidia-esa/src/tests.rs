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

/// upstream: NvidiaESAController.cpp SetZoneColor (4-byte output report, inverted channels)
#[test]
fn report_direct_is_byte_exact() {
    use crate::report;
    assert_eq!(
        report::direct(0, Rgb { r: 0, g: 0, b: 0 }),
        [0x42, 0x0F, 0x0F, 0x0F]
    );
    assert_eq!(
        report::direct(
            4,
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        ),
        [0x46, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        report::direct(2, Rgb { r: 1, g: 2, b: 3 }),
        [0x44, 0x0E, 0x0E, 0x0E]
    );
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
    assert_eq!(crate::devices_table::DETECTORS[0].vid, 0x0955);
    assert_eq!(crate::devices_table::DETECTORS[0].pid, 0x000A);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x0955,
        pid: 0x000A,
        interface: 0,
        usage_page: 0xFFDE,
        usage: 0x0002,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::NvidiaEsa::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 5);
    assert!(h.writes.is_empty());
    let colors = [
        Rgb {
            r: 10,
            g: 20,
            b: 30,
        },
        Rgb {
            r: 40,
            g: 50,
            b: 60,
        },
        Rgb {
            r: 70,
            g: 80,
            b: 90,
        },
        Rgb {
            r: 100,
            g: 110,
            b: 120,
        },
        Rgb {
            r: 130,
            g: 140,
            b: 150,
        },
    ];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 5);
    for (zone, c) in colors.iter().enumerate() {
        assert_eq!(h.writes[zone], crate::report::direct(zone, *c));
    }
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 5);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.vid = 0x1234;
    assert!(!crate::table::identify(&i).supported);
    i = supported_interface();
    i.usage_page = 0x0000;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::NvidiaEsa::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 5);
    assert_eq!(caps.layout.width, 5);
    assert_eq!(caps.layout.height, 1);
    for (idx, led) in caps.layout.leds.iter().enumerate() {
        assert!(led.pos.0 < caps.layout.width);
        assert!(led.pos.1 < caps.layout.height);
        assert_eq!(led.pos.0, idx as u16);
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 4], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 5], &mut h).is_ok());
}
