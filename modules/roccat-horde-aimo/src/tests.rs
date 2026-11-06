// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{ErrorCode, InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1e7d && d.pid == 0x303e));
}

fn info_for() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
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
    get_read: Option<Vec<u8>>,
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
    fn get_feature(&mut self, b: &[u8], len: usize) -> Result<Vec<u8>, ModuleError> {
        let _ = (b, len);
        Ok(self.get_read.take().unwrap_or_default())
    }
}

#[test]
fn init_and_set_colors_are_byte_exact() {
    // upstream: RoccatHordeAimoController.cpp InitialPacket / SetColors
    assert_eq!(&crate::report::init()[..3], &[0x13, 0x08, 0x01]);
    let colors = [
        Rgb { r: 1, g: 2, b: 3 },
        Rgb { r: 4, g: 5, b: 6 },
        Rgb { r: 7, g: 8, b: 9 },
        Rgb { r: 10, g: 11, b: 12 },
        Rgb { r: 13, g: 14, b: 15 },
        Rgb { r: 16, g: 17, b: 18 },
    ];
    let p = crate::report::set_colors(&colors);
    assert_eq!(p.len(), 24);
    assert_eq!(&p[..7], &[0x18, 0x18, 0xff, 0xff, 1, 2, 3]);
    assert_eq!(&p[19..22], &[16, 17, 18]);
    // crc is the (u32) sum of bytes 0..21; only the low two bytes are
    // written, so test the full accumulator truncation.
    let expected: u32 = p[..22].iter().map(|b| u32::from(*b)).sum();
    assert_eq!(u32::from(p[22]) + (u32::from(p[23]) << 8), expected & 0xFFFF);
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::H::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 6);
    assert_eq!(caps.layout.leds[5].name, "LED 6");
    // one init feature report
    assert_eq!(h.features.len(), 1);
    assert_eq!(&h.features[0][..3], &[0x13, 0x08, 0x01],);
    let colors = vec![Rgb { r: 1, g: 0, b: 0 }, Rgb { r: 0, g: 2, b: 0 }];
    colors.iter().for_each(|_| {});
    let mut six = vec![Rgb { r: 0, g: 0, b: 0 }; 6];
    six[0].r = 1;
    six[1].g = 2;
    m.frame(&six, &mut h).unwrap();
    // one 24-byte feature write + one 3-byte feature read per frame
    assert_eq!(h.features.len(), 2);
    assert_eq!(h.features[1].len(), 24);
    assert_eq!(&h.features[1][4..7], &[1, 0, 0]);
    assert_eq!(&h.features[1][7..10], &[0, 2, 0]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 2); // upstream sends no exit packets
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::H::default();
    let mut h = Fake::default();
    let mut i = info_for();
    i.interface = 0;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    let mut i = info_for();
    i.pid = 0x0340;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::H::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert_eq!(led.pos.0, n as u16);
        assert_eq!(led.name, format!("LED {}", n + 1));
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 6], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 5], &mut h).is_err());
}
