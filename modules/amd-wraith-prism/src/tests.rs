// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct RecordingWrite {
    writes: Vec<Vec<u8>>,
}
impl Hid for RecordingWrite {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn report_bytes_are_exact() {
    use crate::report;
    let e = report::enable_direct();
    assert_eq!(e.len(), 65);
    assert_eq!(e[1], 0x41);
    assert_eq!(e[2], 0x03);
    let a = report::apply();
    assert_eq!(a[1], 0x51);
    assert_eq!(a[2], 0x28);
    let colors = [Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }];
    let p = report::packet(&report::LOGO_FAN_IDS, &colors);
    assert_eq!(p[1], 0xC0);
    assert_eq!(p[2], 0x01);
    assert_eq!(p[3], 2);
    assert_eq!(&p[5..9], &[0x00, 1, 2, 3]);
    assert_eq!(&p[9..13], &[0x01, 4, 5, 6]);
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x2516,
        pid: 0x0051,
        interface: 1,
        usage_page: 0xFF00,
        usage: 0,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::AMDWraithPrism::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 17);
    assert_eq!(h.writes.len(), 2);
    assert_eq!(h.writes[0], crate::report::enable_direct());
    assert_eq!(h.writes[1], crate::report::apply());
    let colors: Vec<Rgb> = (0..17)
        .map(|i| Rgb {
            r: i as u8,
            g: (i + 1) as u8,
            b: (i + 2) as u8,
        })
        .collect();
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 4);
    let mut c0 = vec![colors[0], colors[1]];
    let mut c1 = colors[2..17].to_vec();
    assert_eq!(
        h.writes[2],
        crate::report::packet(&crate::report::LOGO_FAN_IDS, &c0)
    );
    assert_eq!(
        h.writes[3],
        crate::report::packet(&crate::report::RING_IDS, &c1)
    );
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 4);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.interface = 0;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::AMDWraithPrism::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.width, 17);
    for (idx, led) in caps.layout.leds.iter().enumerate() {
        assert!(led.pos.0 < caps.layout.width);
        assert_eq!(led.pos.0, idx as u16);
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 16], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 17], &mut h).is_ok());
}
