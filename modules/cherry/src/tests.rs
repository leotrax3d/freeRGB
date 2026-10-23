// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn generated_catalog_matches_upstream() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 25);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x046a && d.interface == 1 && d.usage_page == 0xff1c));
}

fn first_detector() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface as i32,
        usage_page: d.usage_page,
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
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::Cherry::default();
    let mut h = Fake { writes: Vec::new() };
    let caps = m.initialize(&first_detector(), &mut h).unwrap();
    // upstream: RGBController_CherryKeyboard.cpp SetupZones fixed 21x6 matrix
    assert_eq!(caps.layout.leds.len(), 126);
    assert_eq!(caps.layout.width, 21);
    assert_eq!(caps.layout.height, 6);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 126];
    m.frame(&colors, &mut h).unwrap();
    // upstream: CherryKeyboardController.cpp SetKeyboardColors -> SendKeyboardData
    // 378 color bytes in 7 packets of at most 0x36 bytes each, checksummed.
    assert_eq!(h.writes.len(), 7);
    for (n, packet) in h.writes.iter().enumerate() {
        assert_eq!(packet.len(), 64);
        assert_eq!(packet[0], 4);
        assert_eq!(packet[3], 0x0b);
        let offset = n * 0x36;
        assert_eq!(packet[5], offset as u8);
        assert_eq!(packet[6], (offset >> 8) as u8);
        let checksum: u16 = packet[3..].iter().map(|x| u16::from(*x)).sum();
        assert_eq!(packet[1], checksum as u8);
        assert_eq!(packet[2], (checksum >> 8) as u8);
    }
    assert_eq!(h.writes[0][4], 0x36);
    assert_eq!(&h.writes[0][8..11], &[1, 2, 3]);
    assert_eq!(h.writes[6][4], 0x36);
    assert_eq!(h.writes[6][5], 0x44);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 7); // shutdown sends nothing in direct mode
}

#[test]
fn wrong_interface_or_usage_is_not_supported() {
    let mut i = first_detector();
    i.interface = 0; // upstream registers interface 1 with usage page 0xFF1C
    assert!(!crate::table::identify(&i).supported);
    i = first_detector();
    i.usage_page = 0x0001;
    assert!(!crate::table::identify(&i).supported);
    let id = crate::table::identify(&first_detector());
    assert!(id.supported);
}

#[test]
fn layout_leds_are_valid_and_frame_size_is_checked() {
    let mut m = crate::Cherry::default();
    let mut h = Fake { writes: Vec::new() };
    let caps = m.initialize(&first_detector(), &mut h).unwrap();
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert_eq!(led.pos.0, (n % 21) as u16);
        assert_eq!(led.pos.1, (n / 21) as u16);
    }
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 125];
    assert!(m.frame(&colors, &mut h).is_err());
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 126];
    assert!(m.frame(&colors, &mut h).is_ok());
}

#[test]
fn unsupported_device_fails_initialize_without_io() {
    let mut m = crate::Cherry::default();
    let mut h = Fake { writes: Vec::new() };
    let mut i = first_detector();
    i.vid = 0x1234;
    assert!(m.initialize(&i, &mut h).is_err());
    assert!(h.writes.is_empty());
}
