// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: 38 detector rows; the K70 CORE rows (1b1c:1bfd/1bff policy) are
    // not inventoried -> 36 rows; every row routes to Software/Hardware handlers.
    assert_eq!(crate::devices_table::DETECTORS.len(), 36);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1b1c));
    // policy devices must never appear
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.pid != 0x1bfd && d.pid != 0x1bff));
    // the K70 RGB TKL rows exist both as hardware/software handlers
    assert!(crate::devices_table::DETECTORS
        .iter()
        .any(|d| d.handler == "DetectCorsairV2HardwareControllers"));
}

fn info_for(row: usize) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[row];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface as i32,
        usage_page: d.usage_page,
        usage: 0,
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
fn wrong_interface_or_usage_page_is_not_supported() {
    let mut i = info_for(0);
    i.interface = 0; // upstream registers interface 1 with page 0xFF42
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
    let mut m = crate::CorsairPeripheralV2;
    let mut h = Fake;
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    assert!(m.initialize(&info_for(35), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
