// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[derive(Default)]
struct Fake {
    writes: Vec<Vec<u8>>,
    features: Vec<Vec<u8>>,
}
impl Fake {}
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
    // upstream: 9 USB-HID detector rows; the branch version listed 18 (duplicated)
    // and skipped the IPU/usage-page qualifications.
    assert_eq!(crate::devices_table::DETECTORS.len(), 9);
    // ENE (0x0CF2) UniHubs + Strimer, Nuvoton (0x0416) GA II
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.vid == 0x0cf2)
            .count(),
        7
    );
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.vid == 0x0416)
            .count(),
        2
    );
}

fn info_for(row: usize) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[row];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface as i32,
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
fn wrong_interface_is_not_supported() {
    let mut i = info_for(0);
    i.interface = 2; // upstream registers interface 1
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn unsupported_initialize_fails_without_io() {
    let mut m = crate::LianLi;
    let mut h = Fake::default();
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    assert!(m.initialize(&info_for(8), &mut h).is_err());
    assert!(h.writes.is_empty() && h.features.is_empty());
    assert!(m.frame(&[], &mut h).is_err());
}
