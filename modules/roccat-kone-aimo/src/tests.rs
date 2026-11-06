// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 2);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1e7d && (d.pid == 0x2e27 || d.pid == 0x2e2c)));
}

fn info_for(row: usize) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[row];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: i32::from(d.interface),
        usage_page: d.usage_page,
        usage: d.usage,
        product: None,
    }
}

#[derive(Default)]
struct Fake {
    features: Vec<Vec<u8>>,
    reads: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _b: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        let r = if self.reads.is_empty() {
            Vec::new()
        } else {
            self.reads.remove(0)
        };
        Ok(r)
    }
}

#[test]
fn set_colors_buffer_is_byte_exact() {
    let colors: Vec<Rgb> = (0..11)
        .map(|i| Rgb {
            r: i as u8,
            g: i as u8 + 10,
            b: i as u8 + 20,
        })
        .collect();
    let p = crate::report::set_colors(&colors);
    assert_eq!(p.len(), 46);
    assert_eq!(&p[0..2], &[0x0d, 0x2e]);
    // slot layout: LED n -> offset 2+n*4
    for i in 0..11 {
        let b = 2 + i * 4;
        assert_eq!(p[b], i as u8);
        assert_eq!(p[b + 1], i as u8 + 10);
        assert_eq!(p[b + 2], i as u8 + 20);
    }
}

#[test]
fn init_packet_is_byte_exact() {
    assert_eq!(
        crate::report::init(),
        vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]
    );
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 11);
    assert_eq!(caps.layout.leds[0].name, "Wheel LED");
    assert_eq!(caps.layout.leds[4].name, "Strip left LED 4");
    assert_eq!(caps.layout.leds[5].name, "Strip right LED 1");
    assert_eq!(caps.layout.leds[10].name, "Lower right LED");
    assert_eq!(h.features.len(), 1);
    assert_eq!(h.features[0], vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]);
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 11];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
    assert_eq!(h.features[1].len(), 46);
    assert_eq!(&h.features[1][0..2], &[0x0d, 0x2e]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(1), &mut h).unwrap();
    assert_eq!(caps.zones.len(), 5);
    let sum: usize = caps.zones.iter().map(|z| z.leds).sum();
    assert_eq!(sum, 11);
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 11], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 10], &mut h).is_err());
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::K::default();
    let mut h = Fake::default();
    let mut i = info_for(0);
    i.pid = 0x9999;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
}
