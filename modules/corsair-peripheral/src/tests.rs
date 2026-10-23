// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_module_api::sdk::Module;
use freergb_module_api::InterfaceInfo;

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 39);
    // upstream: CorsairPeripheralControllerDetect.cpp row PIDs are unique except
    // intentionally duplicated usage rows; every row routes to one of three handlers.
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1b1c));
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.handler.starts_with("DetectCorsair")));
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
    // Audit result: none of the three handlers has a fully ported Direct-mode path
    // (generic firmware-info/typing, K55 PRO XT key banks, K65 Mini LED map/keepalive).
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
    let mut m = crate::CorsairPeripheral;
    let mut h = Fake;
    // a recognized detector row: still unsupported, still no I/O
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    // unknown VID/PID
    let mut i = info_for(0);
    i.vid = 0x1234;
    assert!(m.initialize(&i, &mut h).is_err());
    // frame is refused when not initialized
    assert!(m.frame(&[], &mut h).is_err());
}

#[test]
fn wrong_interface_or_usage_is_not_supported() {
    let mut i = info_for(0);
    i.interface = 7;
    // Detector rows are matched by VID/PID only in the upstream table; the module
    // reports them as inventoried-unsupported on every interface, never supported.
    assert!(!crate::table::identify(&i).supported);
}
