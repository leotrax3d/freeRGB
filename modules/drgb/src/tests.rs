// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::Module;
use freergb_module_api::InterfaceInfo;

#[test]
fn inventory_is_complete() {
    // upstream: DRGBControllerDetect.cpp registers 33 detectors and
    // DRGBDevices.cpp defines 33 device entries; rows verified by the audit.
    assert_eq!(crate::devices_table::DETECTORS.len(), 33);
    assert_eq!(crate::devices_table::DEVICES.len(), 33);
}

fn info_for(row: usize) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[row];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

#[test]
fn no_device_is_claimed_as_supported() {
    // Audit: every version's Direct-mode path has byte-level defects and the
    // 500 ms keepalive is unported, so nothing may claim support.
    for row in 0..crate::devices_table::DETECTORS.len() {
        let id = crate::table::identify(&info_for(row));
        assert!(!id.supported, "row {} must not claim support", row);
        assert!(id.reason.as_deref().unwrap().contains("not ported"));
    }
}

#[test]
fn unsupported_initialize_fails_without_io() {
    use freergb_module_api::sdk::Hid;
    use freergb_module_api::ModuleError;
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
    let mut m = crate::Drgb;
    let mut h = Fake;
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    assert!(m.initialize(&info_for(32), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
