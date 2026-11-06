// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[derive(Default)]
struct Fake {
    writes: Vec<Vec<u8>>,
    features: Vec<Vec<u8>>,
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
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn inventory_is_complete() {
    // upstream: 5 unique USB-HID detector rows (IT8297 VID 0x048D PIDs 0x8297/
    // 0x8950/0x5702/0x5711 + GC 0x0414:0xa100), all via the DETECTOR_NAME define.
    assert_eq!(crate::devices_table::DETECTORS.len(), 5);
    for pid in [0x8297u16, 0x8950, 0x5702, 0x5711, 0xa100] {
        assert!(crate::devices_table::DETECTORS.iter().any(|d| d.pid == pid));
    }
    for d in crate::devices_table::DETECTORS.iter() {
        assert_eq!((d.usage_page, d.usage), (0xff89, 0x00cc));
        assert_eq!(d.name, "Gigabyte RGB Fusion 2 USB");
    }
}

fn info_for(row: usize) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[row];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: 0,
        usage_page: d.usage_page,
        usage: d.usage,
        product: None,
    }
}

#[test]
fn no_device_is_claimed_as_supported() {
    for row in 0..crate::devices_table::DETECTORS.len() {
        let id = crate::table::identify(&info_for(row));
        assert!(!id.supported, "row {} must not claim support", row);
        assert!(id.reason.as_deref().unwrap().contains("not ported"));
    }
}

#[test]
fn wrong_usage_is_not_supported() {
    let mut i = info_for(0);
    i.usage_page = 0x0001;
    assert!(!crate::table::identify(&i).supported);
    let mut i2 = info_for(0);
    i2.usage = 0x0001;
    assert!(!crate::table::identify(&i2).supported);
}

#[test]
fn unsupported_initialize_fails_without_io() {
    let mut m = crate::RgbFusion2;
    let mut h = Fake::default();
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    assert!(h.writes.is_empty() && h.features.is_empty());
    assert!(m.frame(&[], &mut h).is_err());
}
