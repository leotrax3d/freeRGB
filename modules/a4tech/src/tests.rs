// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: 5 mouse rows (0xFF33 except MP 50RS on 0xFFF2) + the B820R row
    // (interface 2, usage page 0xFF52, usage 0x0210).
    assert_eq!(crate::devices_table::DETECTORS.len(), 6);
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.protocol == "mouse")
            .count(),
        5
    );
    let b820 = &crate::devices_table::DETECTORS[5];
    assert_eq!(b820.pid, 0xfa10);
    assert_eq!(b820.interface, 2);
    assert_eq!(b820.usage_page, 0xff52);
    assert_eq!(b820.usage, 0x0210);
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
    i.interface = 1; // upstream registers interface 2
    assert!(!crate::table::identify(&i).supported);
    let mut i2 = info_for(0);
    i2.usage_page = 0x0001;
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
    let mut m = crate::A4Tech;
    let mut h = Fake;
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    assert!(m.initialize(&info_for(5), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
