// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 3);
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.pid == pid)
        .unwrap();
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
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn frame_writes_19_leds_with_two_reports() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x18d9), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 19);
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 19];
    colors[0] = Rgb { r: 1, g: 2, b: 3 };
    colors[15] = Rgb { r: 4, g: 5, b: 6 };
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2);
    assert_eq!(h.writes[0][0], 0x00);
    assert_eq!(h.writes[0][2], 0x81);
    assert_eq!(&h.writes[0][6..9], &[1, 2, 3]);
    assert_eq!(h.writes[1][3], 0x01);
    assert_eq!(&h.writes[1][6..9], &[4, 5, 6]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x18c5), &mut h).unwrap();
    for led in &caps.layout.leds {
        assert!(!led.name.is_empty());
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 19], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 18], &mut h).is_err());
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let mut i = info_for(0x18d9);
    i.vid = 0x1234;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
}
