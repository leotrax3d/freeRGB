// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: AresonControllerDetect.cpp registers 6 IPU rows on
    // interface 1, usage page 0xFF02, usage 2.
    assert_eq!(crate::devices_table::DETECTORS.len(), 6);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x25a7 && d.interface == 1 && d.usage_page == 0xff02 && d.usage == 2));
}

fn first_detector() -> InterfaceInfo {
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
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::A::default();
    let mut h = Fake {
        features: Vec::new(),
    };
    let caps = m.initialize(&first_detector(), &mut h).unwrap();
    // upstream: RGBController_Areson.cpp has a single-LED logo zone
    assert_eq!(caps.layout.leds.len(), 1);
    assert!(h.features.is_empty()); // initialization sends nothing
    m.frame(&[freergb_core::Rgb { r: 1, g: 2, b: 3 }], &mut h)
        .unwrap();
    assert_eq!(h.features.len(), 1);
    // upstream: AresonController.cpp SetMode, 17-byte feature report
    assert_eq!(h.features[0].len(), 17);
    assert_eq!(h.features[0][0], 0x08);
    assert_eq!(&h.features[0][6..13], &[0x02, 1, 2, 3, 0x00, 0xff, 0x4e]);
    assert_eq!(h.features[0][16], 0x4a);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 1); // upstream sends no exit packets
}

#[test]
fn wrong_interface_or_usage_is_not_supported() {
    let mut i = first_detector();
    i.interface = 0;
    assert!(!crate::table::identify(&i).supported);
    i = first_detector();
    i.usage = 1;
    assert!(!crate::table::identify(&i).supported);
    i = first_detector();
    i.vid = 0x1234;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::A::default();
    let mut h = Fake {
        features: Vec::new(),
    };
    let caps = m.initialize(&first_detector(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    assert!(m
        .frame(&[freergb_core::Rgb { r: 0, g: 0, b: 0 }; 2], &mut h)
        .is_err());
    assert!(m
        .frame(&[freergb_core::Rgb { r: 0, g: 0, b: 0 }], &mut h)
        .is_ok());
    assert_eq!(h.features.len(), 1);
}
