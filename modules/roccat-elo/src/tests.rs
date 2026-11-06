// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};
#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1e7d && d.pid == 0x3a34));
}

fn info_for() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: i32::from(d.interface),
        usage_page: d.usage_page,
        usage: d.usage,
        product: None,
    }
}

#[derive(Default)]
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
fn init_and_direct_packets_are_byte_exact() {
    // upstream: RoccatEloController.cpp SendInit / SendDirect
    let init = crate::report::init();
    assert_eq!(init.len(), 5);
    assert_eq!(&init[0][..2], &[0xff, 0x01]);
    assert_eq!(&init[1][..2], &[0xff, 0x02]);
    assert_eq!(&init[2][..4], &[0xff, 0x03, 0x00, 0x01]);
    // fourth packet is SendDirect(0)
    assert_eq!(&init[3][..7], &[0xff, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(&init[4][..2], &[0xff, 0x01]);
    let p = crate::report::direct(&Rgb { r: 1, g: 2, b: 3 });
    assert_eq!(&p[..7], &[0xff, 0x04, 0, 0, 1, 2, 3]);
    assert!(&p[7..].iter().all(|b| *b == 0));
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::E::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    assert_eq!(h.writes.len(), 5);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 1];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 6);
    assert_eq!(h.writes[5][..7], [0xff, 0x04, 0, 0, 1, 2, 3]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 6); // upstream sends no exit packets
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::E::default();
    let mut h = Fake::default();
    let mut i = info_for();
    i.interface = 0;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    let mut i = info_for();
    i.pid = 0x9999;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::E::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert_eq!(caps.layout.leds[0].pos, (0, 0));
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 1];
    assert!(m.frame(&colors, &mut h).is_ok());
    assert!(m.frame(&[], &mut h).is_err());
}
