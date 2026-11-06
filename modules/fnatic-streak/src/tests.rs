// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: FnaticStreakControllerDetect.cpp registers 3 _I rows on
    // interface 1; LED totals from GetLEDCount (mini 106, full 124) and the
    // Streak65 direct path (65).
    assert_eq!(crate::devices_table::DETECTORS.len(), 3);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x2f0e && d.interface == 1));
    let c = |p: u16| {
        crate::devices_table::DETECTORS
            .iter()
            .find(|d| d.pid == p)
            .unwrap()
    };
    let s65 = c(0x0105);
    assert_eq!(s65.protocol, "65");
    assert!(s65.leds >= 65);
}

fn first_detector() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface as i32,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

#[test]
fn no_device_is_claimed_as_supported() {
    for row in 0..crate::devices_table::DETECTORS.len() {
        let d = &crate::devices_table::DETECTORS[row];
        let i = InterfaceInfo {
            vid: d.vid,
            pid: d.pid,
            interface: d.interface as i32,
            usage_page: 0,
            usage: 0,
            product: None,
        };
        let id = crate::table::identify(&i);
        assert!(!id.supported, "row {} must not claim support", row);
        assert!(id.reason.as_deref().unwrap().contains("not ported"));
    }
}

#[test]
fn wrong_interface_is_not_supported() {
    let mut i = first_detector();
    i.interface = 0; // upstream registers interface 1
    assert!(!crate::table::identify(&i).supported);
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
    let mut m = crate::A;
    let mut h = Fake;
    assert!(m.initialize(&first_detector(), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
