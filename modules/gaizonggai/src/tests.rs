// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: 9 REGISTER_HID_DETECTOR_I rows on interface 3; zone LED counts
    // from RGBController_GaiZhongGai.cpp SetupZones (hubs are device-derived).
    assert_eq!(crate::devices_table::DETECTORS.len(), 9);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x3061 && d.interface == 3));
    let leds = |pid: u16| {
        crate::devices_table::DETECTORS
            .iter()
            .find(|d| d.pid == pid)
            .unwrap()
            .leds
    };
    assert_eq!(leds(0x4700), 68); // 68 PRO
    assert_eq!(leds(0x4701), 42); // 42 PRO
    assert_eq!(leds(0x4770), 88); // 17+Touch
    assert_eq!(leds(0x4771), 85); // 17 PRO
    assert_eq!(leds(0x4772), 88); // 20 PRO
    assert_eq!(leds(0x4720), 89); // Dial
    assert_eq!(leds(0x4710), 0); // LightBoard needs flash reads
    assert_eq!(leds(0x4711), 637); // RGB HUB Green upstream maximum
    assert_eq!(leds(0x4712), 633); // RGB HUB Blue upstream maximum
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid,
        interface: d.interface as i32,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

#[test]
fn no_device_is_claimed_as_supported() {
    for d in crate::devices_table::DETECTORS.iter() {
        let id = crate::table::identify(&info_for(d.pid));
        assert!(!id.supported, "{} must not claim support", d.name);
        assert!(id.reason.as_deref().unwrap().contains("not ported"));
    }
}

#[test]
fn wrong_interface_or_unknown_pid_is_not_supported() {
    let mut i = info_for(0x4700);
    i.interface = 1; // upstream registers interface 3
    assert!(!crate::table::identify(&i).supported);
    let mut i2 = info_for(0x4700);
    i2.pid = 0x9999;
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
    let mut m = crate::GaiZhongGai;
    let mut h = Fake;
    assert!(m.initialize(&info_for(0x4700), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
