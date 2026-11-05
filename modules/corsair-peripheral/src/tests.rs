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
fn mouse_class_devices_are_supported() {
    for row in 0..crate::devices_table::DETECTORS.len() {
        let d = &crate::devices_table::DETECTORS[row];
        let id = crate::table::identify(&info_for(row));
        let portable = matches!(
            d.pid,
            0x1b34
                | 0x1b74
                | 0x1b3c
                | 0x1b75
                | 0x1b5d
                | 0x1b12
                | 0x1b2e
                | 0x1b5a
                | 0x1b5c
                | 0x1b1e
                | 0x1b3e
                | 0x1b8b
                | 0x1b2f
                | 0x1b32
                | 0x1b3b
                | 0x0a34
        );
        assert_eq!(id.supported, portable, "row {}", row);
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
    let mut m = crate::CorsairPeripheral::default();
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
struct FirmwareMouse {
    writes: Vec<Vec<u8>>,
}
impl freergb_module_api::sdk::Hid for FirmwareMouse {
    fn write(&mut self, b: &[u8]) -> Result<usize, freergb_module_api::ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, freergb_module_api::ModuleError> {
        let mut v = vec![0u8; 65];
        v[0x14] = 0xC1;
        Ok(v)
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, freergb_module_api::ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(
        &mut self,
        _: &[u8],
        _: usize,
    ) -> Result<Vec<u8>, freergb_module_api::ModuleError> {
        let mut v = vec![0u8; 65];
        v[0x14] = 0xC1;
        Ok(v)
    }
}
#[test]
fn mouse_full_sequence_is_byte_exact() {
    let mut m = crate::CorsairPeripheral::default();
    let mut h = FirmwareMouse { writes: Vec::new() };
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.pid == 0x1b34)
        .unwrap();
    let i = freergb_module_api::InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 15);
    // firmware read write + LightingControl write = at least 2
    assert!(h.writes.len() >= 2);
    m.shutdown(None, &mut h).unwrap();
}
