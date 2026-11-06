// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: 4 keyboard rows (interface 1, usage page 0x08, usage 0x4B),
    // 2 X20 rows (0xFFFF), 1 X12 row (usage page 0x0008, usage 0x4B).
    assert_eq!(crate::devices_table::DETECTORS.len(), 7);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x3842));
    let x12 = &crate::devices_table::DETECTORS[6];
    assert_eq!(x12.protocol, "x12");
    assert_eq!(x12.pid, 0x2422);
    assert_eq!(x12.interface, 1);
    assert_eq!(x12.usage_page, 0x0008);
    assert_eq!(x12.usage, 0x4b);
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.protocol == "keyboard")
            .count(),
        4
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
fn wrong_interface_or_usage_is_not_supported() {
    let mut i = info_for(0);
    i.interface = 2;
    assert!(!crate::table::identify(&i).supported);
    let mut i2 = info_for(0);
    i2.usage_page = 0x0012;
    assert!(!crate::table::identify(&i2).supported);
}

#[test]
fn unsupported_initialize_fails_without_io() {
    struct Fake;
    impl Hid for Fake {
        fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
            panic!("no device I/O may happen for unsupported devices")
        }
        fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
            panic!("no device I/O may happen for unsupported devices")
        }
        fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
            panic!("no device I/O may happen for unsupported devices")
        }
        fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
            panic!("no device I/O may happen for unsupported devices")
        }
    }
    let mut m = crate::Evga;
    let mut h = Fake;
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    assert!(m.initialize(&info_for(4), &mut h).is_err());
    assert!(m.initialize(&info_for(6), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
