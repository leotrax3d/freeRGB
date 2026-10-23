// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

fn keyboard() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface as i32,
        usage_page: d.usage_page,
        usage: d.usage,
        product: None,
    }
}

struct Fake {
    features: Vec<Vec<u8>>,
    writes: Vec<Vec<u8>>,
}
impl Hid for Fake {
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
fn inventory_is_complete() {
    // upstream: GigabyteAorusLaptopControllerDetect.cpp registers 4 IPU rows on
    // interface 3, usage page 0xFF01, usage 1; two are keyboards (direct mode)
    // and two are backlight controllers (no direct mode).
    assert_eq!(crate::devices_table::DETECTORS.len(), 4);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x0414 && d.interface == 3 && d.usage_page == 0xff01 && d.usage == 1));
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.keyboard)
            .count(),
        2
    );
}

#[test]
fn backlight_devices_are_not_supported() {
    // upstream: RGBController_GigabyteAorusLaptop.cpp adds the Direct mode only
    // for GIGABYTE_AORUS_LAPTOP_KEYBOARD_TYPE.
    let backlight = &crate::devices_table::DETECTORS[1];
    assert!(!backlight.keyboard);
    let i = InterfaceInfo {
        vid: backlight.vid,
        pid: backlight.pid,
        interface: backlight.interface as i32,
        usage_page: backlight.usage_page,
        usage: backlight.usage,
        product: None,
    };
    let id = crate::table::identify(&i);
    assert!(!id.supported);
    assert!(id.reason.as_deref().unwrap().contains("not supported"));
    let mut m = crate::Aorus::default();
    let mut h = Fake {
        features: Vec::new(),
        writes: Vec::new(),
    };
    assert!(m.initialize(&i, &mut h).is_err());
    assert!(h.features.is_empty());
}

#[test]
fn keyboard_initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::Aorus::default();
    let mut h = Fake {
        features: Vec::new(),
        writes: Vec::new(),
    };
    let caps = m.initialize(&keyboard(), &mut h).unwrap();
    // upstream: single-LED zone for the whole keyboard; no init packets
    assert_eq!(caps.layout.leds.len(), 1);
    assert!(h.features.is_empty());
    assert!(h.writes.is_empty()); // feature reports, not output writes
    m.frame(&[Rgb { r: 1, g: 2, b: 3 }], &mut h).unwrap();
    assert_eq!(h.features.len(), 1);
    // upstream: SetDirect feature report, 9 bytes, default brightness 0x19
    assert_eq!(h.features[0][..], [0, 0x08, 0x01, 1, 2, 3, 0x19, 0, 0xd8]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 1); // upstream sends no exit packets
}

#[test]
fn wrong_interface_or_usage_is_not_supported() {
    let mut i = keyboard();
    i.interface = 2;
    assert!(!crate::table::identify(&i).supported);
    let i2 = InterfaceInfo {
        vid: 0x0414,
        pid: 0x9999,
        interface: 3,
        usage_page: 0xff01,
        usage: 1,
        product: None,
    };
    assert!(!crate::table::identify(&i2).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::Aorus::default();
    let mut h = Fake {
        features: Vec::new(),
        writes: Vec::new(),
    };
    let caps = m.initialize(&keyboard(), &mut h).unwrap();
    assert_eq!(caps.layout.leds[0].pos, (0, 0));
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 2], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }], &mut h).is_ok());
    assert_eq!(h.features.len(), 1);
}
