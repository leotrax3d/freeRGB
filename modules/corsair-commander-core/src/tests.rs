// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::Module;
use freergb_module_api::InterfaceInfo;

#[test]
fn inventory_is_complete() {
    // upstream: CorsairCommanderCoreController.cpp defines 7 PIDs, the detector
    // registers each once with usage page 0xFF42 and usage 1 on interface 0.
    assert_eq!(crate::devices_table::DETECTORS.len(), 7);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1b1c && d.interface == 0 && d.usage_page == 0xff42 && d.usage == 1));
    assert!(crate::devices_table::DETECTORS
        .iter()
        .any(|d| d.name.contains("XT") && d.pid == 0x0c2a));
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
    let mut m = crate::CommanderCore;
    let mut h = Fake;
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    assert!(m.initialize(&info_for(6), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
