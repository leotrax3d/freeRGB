// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
#[derive(Default)]
struct Fake {
    writes: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(p.to_vec());
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
fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1038,
        pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}
#[test]
fn inventory_is_complete() {
    assert_eq!(devices_table::DETECTORS.len(), 76)
}
#[test]
fn direct_report_is_byte_exact() {
    assert_eq!(
        &report::direct(&[Rgb { r: 1, g: 2, b: 3 }])[..6],
        &[0, 2, 1, 2, 3, 0]
    );
}
#[test]
fn lifecycle_uses_brokered_hid() {
    let mut m = SteelSeries::default();
    let mut h = Fake::default();
    let c = m
        .initialize(&iface(devices_table::DETECTORS[0].pid), &mut h)
        .unwrap();
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; c.layout.leds.len()];
    m.frame(&colors, &mut h).unwrap();
    assert!(!h.writes.is_empty());
}
