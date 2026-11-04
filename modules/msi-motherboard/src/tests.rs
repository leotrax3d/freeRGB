// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1462,
        pid,
        interface: 1,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

/// Fake HID with configurable 0x52 probe answer and firmware responses.
struct Fake {
    probe_len: usize,
    writes: Vec<Vec<u8>>,
    features: Vec<Vec<u8>>,
    reads: usize,
}

impl Fake {
    fn new(probe_len: usize) -> Self {
        Fake {
            probe_len,
            writes: Vec::new(),
            features: Vec::new(),
            reads: 0,
        }
    }
}

impl Hid for Fake {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        self.reads += 1;
        let mut r = vec![0u8; 64];
        r[2] = 0x25;
        Ok(r)
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], len: usize) -> Result<Vec<u8>, ModuleError> {
        Ok(vec![0u8; self.probe_len.min(len)])
    }
}

fn first_detector() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    iface(d.pid)
}

#[test]
fn inventory_is_complete() {
    // upstream: MSIMotherboardControllerDetect.cpp registers one 0x52-probe
    // handler for every MSI board PID (interface 1, usage page 0)
    assert!(crate::devices_table::DETECTORS.len() > 120);
}

#[test]
fn unsupported_interface_is_not_identified() {
    let mut i = first_detector();
    i.interface = 2;
    let r = crate::table::identify(&i);
    assert!(!r.supported);
}

#[test]
fn probe_selects_only_the_112_byte_variant() {
    let mut m = crate::MsiMotherboard::default();
    let mut h = Fake::new(112);
    let caps = m.initialize(&first_detector(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 10);
    // upstream: ReadFwVersion sends 0xB0 and 0xB6 requests
    assert_eq!(&h.writes[0][0..2], &[0x01, 0xB0]);
    assert_eq!(&h.writes[1][0..2], &[0x01, 0xB6]);
    assert!(h.writes.iter().all(|p| p.len() == 64));
    assert!(h
        .writes
        .iter()
        .all(|p| p.iter().skip(2).all(|b| *b == 0xCC)));
    assert_eq!(h.reads, 2);
    // upstream: ReadSettings performs a 112-byte 0x52 get report
    assert_eq!(caps.firmware.as_deref(), Some("APROM: 2.5, LDROM: 2.5"));
    // zones: JRGB1, JRAINBOW1, JCORSAIR, Onboard LEDs x7
    assert_eq!(caps.zones.len(), 4);
    assert_eq!(caps.zones[3].leds, 7);
    m.shutdown(None, &mut h).unwrap();
}

fn frame_bytes(probe_len: usize, colors: &[Rgb]) -> Result<Vec<Vec<u8>>, ModuleError> {
    let mut m = crate::MsiMotherboard::default();
    let mut h = Fake::new(probe_len);
    m.initialize(&first_detector(), &mut h).unwrap();
    m.frame(colors, &mut h)?;
    Ok(h.features)
}

#[test]
fn frame_sends_single_112_byte_packet() {
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 10];
    let p = frame_bytes(112, &colors).unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].len(), 112);
    assert_eq!(p[0][0], 0x52);
    assert_eq!(p[0][111], 0);
}

#[test]
fn non_112_variants_fail_closed() {
    for probe in [162usize, 185, 300] {
        let mut m = crate::MsiMotherboard::default();
        let mut h = Fake::new(probe);
        assert!(
            m.initialize(&first_detector(), &mut h).is_err(),
            "{}-byte probe must not initialize",
            probe
        );
    }
}

#[test]
fn direct_packet_layouts_are_valid_and_byte_exact() {
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 10];
    let p = frame_bytes(112, &colors).unwrap()[0].clone();
    // j_rgb_1
    assert_eq!(&p[1..11], &[1, 1, 2, 3, 0x28, 1, 2, 3, 0x80, 0]);
    // j_corsair_outerll120 untouched besides effect
    assert_eq!(&p[31..41], &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // onboard master stays black
    assert_eq!(&p[41..51], &[1, 0, 0, 0, 0x28, 0, 0, 0, 0x80, 0]);
    // on_board_led_1..6 filled from frame colors 3..9
    assert_eq!(&p[51..61], &[1, 1, 2, 3, 0x28, 1, 2, 3, 0x80, 0]);
    assert_eq!(&p[101..111], &[1, 1, 2, 3, 0x28, 1, 2, 3, 0x80, 0]);
    // frame length checks
    let mut m = crate::MsiMotherboard::default();
    let mut h = Fake::new(112);
    m.initialize(&first_detector(), &mut h).unwrap();
    assert!(m.frame(&colors[..9], &mut h).is_err());
    m.frame(&colors, &mut h).unwrap();
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::MsiMotherboard::default();
    let mut h = Fake::new(112);
    let caps = m.initialize(&first_detector(), &mut h).unwrap();
    assert_eq!(
        caps.zones.iter().map(|z| z.leds).sum::<usize>(),
        caps.layout.leds.len()
    );
    let mut seen = std::collections::HashSet::new();
    for led in &caps.layout.leds {
        assert!(!led.name.is_empty());
        assert!(seen.insert(led.pos));
    }
}
