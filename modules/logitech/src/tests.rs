// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
use freergb_core::Rgb;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{InterfaceInfo, ModuleError};
#[derive(Default)]
struct Fake {
    w: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.w.push(p.to_vec());
        Ok(p.len())
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
fn inventory() {
    assert_eq!(devices_table::DETECTORS.len(), 55)
}
#[test]
fn bytes() {
    assert_eq!(
        &report::direct(&[Rgb { r: 1, g: 2, b: 3 }])[..4],
        &[2, 1, 2, 3]
    );
    assert_eq!(
        &report::g213(2, Rgb { r: 1, g: 2, b: 3 })[..10],
        &[0x11, 0xff, 0x0c, 0x3a, 2, 1, 1, 2, 3, 2]
    );
    assert_eq!(
        &report::g203(&[Rgb { r: 1, g: 2, b: 3 }])[..9],
        &[0x11, 0xff, 0x12, 0x10, 1, 1, 2, 3, 0]
    );
    assert_eq!(
        &report::g815_little(0x04, Rgb { r: 1, g: 2, b: 3 })[..8],
        &[0x11, 0xff, 0x10, 0x1f, 4, 1, 2, 3]
    );
    assert_eq!(
        &report::lightsync(0x3a, 0, Rgb { r: 1, g: 2, b: 3 })[..10],
        &[0x11, 1, 0x0e, 0x3a, 0, 1, 1, 2, 3, 2]
    );
    assert_eq!(
        &report::x56_color(Rgb { r: 1, g: 2, b: 3 })[..6],
        &[9, 0, 3, 1, 2, 3]
    );
    assert_eq!(
        &report::g933(1, Rgb { r: 1, g: 2, b: 3 })[..10],
        &[0x11, 0xff, 4, 0x3a, 1, 1, 1, 2, 3, 2]
    );
}
#[test]
fn lifecycle() {
    let d = &devices_table::DETECTORS[0];
    let i = InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let mut m = Logitech::default();
    let mut h = Fake::default();
    let c = m.initialize(&i, &mut h).unwrap();
    m.frame(&vec![Rgb { r: 1, g: 2, b: 3 }; c.layout.leds.len()], &mut h)
        .unwrap();
    assert!(!h.w.is_empty())
}
