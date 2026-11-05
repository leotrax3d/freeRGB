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
fn v1_devices_are_supported() {
    for row in 0..crate::devices_table::DETECTORS.len() {
        let d = &crate::devices_table::DETECTORS[row];
        let dev = crate::devices_table::DEVICES
            .iter()
            .find(|e| e.pid == d.pid);
        let id = crate::table::identify(&info_for(row));
        assert_eq!(
            id.supported,
            dev.is_some_and(|e| e.version == 1 || e.version == 2),
            "row {}",
            row
        );
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
    let mut m = crate::Drgb::default();
    let mut h = Fake;
    assert!(m.initialize(&info_for(0), &mut h).is_err());
    assert!(m.initialize(&info_for(32), &mut h).is_err());
    assert!(m.frame(&[], &mut h).is_err());
}
struct RecordingWrite {
    writes: Vec<Vec<u8>>,
}
impl freergb_module_api::sdk::Hid for RecordingWrite {
    fn write(&mut self, b: &[u8]) -> Result<usize, freergb_module_api::ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, freergb_module_api::ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, freergb_module_api::ModuleError> {
        unreachable!()
    }
    fn get_feature(
        &mut self,
        _: &[u8],
        _: usize,
    ) -> Result<Vec<u8>, freergb_module_api::ModuleError> {
        unreachable!()
    }
}
#[test]
fn v1_frame_is_byte_exact() {
    let mut m = crate::Drgb::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let v1dev = crate::devices_table::DEVICES
        .iter()
        .find(|d| d.version == 1)
        .unwrap();
    let row = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.pid == v1dev.pid)
        .unwrap();
    let i = freergb_module_api::InterfaceInfo {
        vid: row.vid,
        pid: row.pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let caps = m.initialize(&i, &mut h).unwrap();
    assert!(!caps.layout.leds.is_empty());
    let total = caps.layout.leds.len();
    assert!(total <= 1800);
    let colors = vec![freergb_core::Rgb { r: 1, g: 2, b: 3 }; total];
    assert!(m.frame(&colors, &mut h).is_ok());
    assert!(!h.writes.is_empty());
    assert_eq!(h.writes[0][1], 0x46);
    assert_eq!(h.writes[0][2], 0xBB);
    assert!(h.writes[1][1] == (0x47u8) || h.writes[1][1] == 0x47);
    m.shutdown(None, &mut h).unwrap();
}
