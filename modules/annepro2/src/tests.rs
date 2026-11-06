// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

fn first_detector() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface as i32,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

struct Fake {
    writes: Vec<Vec<u8>>,
}
impl Hid for Fake {
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
fn inventory_is_complete() {
    // upstream: AnnePro2ControllerDetect.cpp registers 5 _I rows on interface 1
    // (the fifth with VID 0x3311).
    assert_eq!(crate::devices_table::DETECTORS.len(), 5);
    assert_eq!(crate::devices_table::DETECTORS[4].vid, 0x3311);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.interface == 1));
    // upstream: RGBController_AnnePro2.cpp led_names[] has 61 entries
    assert_eq!(crate::led_names::LED_NAMES.len(), 61);
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::A::default();
    let mut h = Fake { writes: Vec::new() };
    let caps = m.initialize(&first_detector(), &mut h).unwrap();
    // upstream: 5x14 matrix, 61 real LEDs
    assert_eq!(caps.layout.leds.len(), 61);
    assert_eq!(caps.layout.width, 14);
    assert_eq!(caps.layout.height, 5);
    assert!(h.writes.is_empty()); // no init packets upstream
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 61];
    colors[0] = Rgb { r: 1, g: 2, b: 3 };
    colors[40] = Rgb { r: 4, g: 5, b: 6 };
    m.frame(&colors, &mut h).unwrap();
    // 210 real bytes -> 5 packets like upstream SendDirect
    assert_eq!(h.writes.len(), 5);
    assert_eq!(
        h.writes[0][..13],
        [0, 0x7b, 0x10, 0x41, 0x50, 55, 0, 0, 0x7d, 32, 3, 0xff, 2]
    );
    // real frame: LED 0 at byte 0, then zero-filled LEDs; LED 40 written at real
    // index 40 (real 41 stays a zero gap triplet), packet 2 covers real 102..153
    assert!(h.writes.iter().all(|p| p.len() == 64));
    let p2 = &h.writes[2];
    assert_eq!(&p2[13 + 18..13 + 21], &[4, 5, 6]); // LED 40 at real 40
    assert_eq!(&p2[13 + 21..13 + 24], &[0, 0, 0]); // consumed gap triplet
    assert_eq!(&p2[13 + 24..13 + 27], &[0, 0, 0]); // LED 41 (black)
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 5); // no exit packets
}

#[test]
fn wrong_interface_is_not_supported() {
    let mut i = first_detector();
    i.interface = 0; // upstream registers interface 1
    assert!(!crate::table::identify(&i).supported);
    let mut i2 = first_detector();
    i2.pid = 0x9999;
    assert!(!crate::table::identify(&i2).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::A::default();
    let mut h = Fake { writes: Vec::new() };
    let caps = m.initialize(&first_detector(), &mut h).unwrap();
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert_eq!(led.pos.0, (n % 14) as u16);
        assert_eq!(led.pos.1, (n / 14) as u16);
        assert!(!led.name.is_empty());
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 60], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 61], &mut h).is_ok());
    assert_eq!(h.writes.len(), 5);
}
