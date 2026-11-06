// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 2);
}

fn info_for(row: usize) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[row];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface,
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
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> { unreachable!() }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> { unreachable!() }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> { unreachable!() }
}

#[test]
fn packet_is_byte_exact() {
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 15];
    colors[0] = Rgb { r: 9, g: 8, b: 7 };
    let p = crate::report::direct(&colors);
    assert_eq!(p.len(), 65);
    assert_eq!(&p[..5], &[0xee, 0xc0, 0x81, 0, 0]);
    // i=0 group: p[5]=0, p[6..9]=[9,8,7]
    assert_eq!(&p[5..9], &[0, 9, 8, 7]);
}
#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 15);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 15];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 1);
    assert_eq!(h.writes[0][0], 0xee);
    assert_eq!(h.writes[0][1], 0xc0);
    assert_eq!(h.writes[0][2], 0x81);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 1);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(1), &mut h).unwrap();
    for led in &caps.layout.leds {
        assert!(!led.name.is_empty());
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 15], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 14], &mut h).is_err());
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let mut i = info_for(0);
    i.pid = 0x1899;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    let mut i = info_for(0);
    i.usage_page = 0xff07;
    assert!(!crate::table::identify(&i).supported);
}
