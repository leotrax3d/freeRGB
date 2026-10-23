// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::Hid;
use freergb_module_api::sdk::Module;
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: HyperXMouseControllerDetect.cpp registers 14 IP/PU/IPU rows with
    // distinct interface/usage-page qualifications (ANY = 255).
    assert_eq!(crate::devices_table::DETECTORS.len(), 14);
    assert_eq!(crate::devices_table::DEVICES.len(), 14);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x0951 || d.vid == 0x03f0));
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid,
        interface: 1,
        usage_page: 0xff01,
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
            usage_page: d.usage_page,
            usage: d.usage,
            product: None,
        };
        let id = crate::table::identify(&i);
        assert!(!id.supported, "row {} must not claim support", row);
        assert!(id.reason.as_deref().unwrap().contains("not ported"));
    }
}

#[test]
fn wrong_interface_or_usage_page_is_not_supported() {
    // Surge: interface 1, usage page 0xFF01. A control interface must not match.
    let i = info_for(0x16d3);
    let id = crate::table::identify(&info_for(0x16d3));
    assert!(id.model.as_deref().is_some_and(|m| m.contains("Surge")));
    let mut i2 = i.clone();
    i2.interface = 0;
    assert!(!crate::table::identify(&i2).supported);
    let mut i3 = i;
    i3.usage_page = 0x0001;
    assert!(!crate::table::identify(&i3).supported);
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
    let mut m = crate::HyperX;
    let mut h = Fake;
    assert!(m.initialize(&info_for(0x16d3), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
