// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
use freergb_core::Rgb;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{InterfaceInfo, ModuleError, Zone};
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
    assert_eq!(devices_table::DETECTORS.len(), 51);
}
#[test]
fn packets_are_byte_exact() {
    let p = report::base(0, &[Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&p[..8], &[0, 0x32, 0x52, 1, 0x18, 2, 1, 3]);
    let q = report::quad(4, &[Rgb { r: 4, g: 5, b: 6 }]);
    assert_eq!(&q[..8], &[0, 0x32, 0x52, 5, 0x24, 5, 4, 6]);
    let t = report::trio(2, 1, &[Rgb { r: 7, g: 8, b: 9 }]);
    assert_eq!(&t[..11], &[0, 0x32, 0x52, 3, 0x24, 3, 2, 0, 8, 7, 9]);
}
#[test]
fn lifecycle_uses_zones() {
    let d = &devices_table::DETECTORS[0];
    let i = InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let mut m = Thermaltake::default();
    let mut h = Fake { writes: Vec::new() };
    let c = m
        .initialize_with_zones(
            &i,
            Some(&[
                Zone {
                    name: "A".into(),
                    leds: 3,
                },
                Zone {
                    name: "B".into(),
                    leds: 2,
                },
            ]),
            &mut h,
        )
        .unwrap();
    assert_eq!(c.layout.leds.len(), 8);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 8];
    m.frame(&colors, &mut h).unwrap();
    assert!(h.writes.len() >= 6);
}

#[test]
fn unknown_device_is_not_supported_with_reason() {
    let i = InterfaceInfo {
        vid: 0x1234,
        pid: 0x5678,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let id = table::identify(&i);
    assert!(!id.supported);
    assert!(id.reason.is_some());
}
