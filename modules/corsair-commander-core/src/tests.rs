// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1b1c,
        pid,
        interface: 0,
        usage_page: 0xff42,
        usage: 1,
        product: None,
    }
}

/// Fake HID returning canned read answers.
struct Fake {
    writes: Vec<Vec<u8>>,
    answers: Vec<Vec<u8>>,
}
impl Fake {
    fn new() -> Self {
        Fake {
            writes: Vec::new(),
            answers: vec![vec![0u8; 97]],
        }
    }
    fn with_read_answer(mut self, answer: Vec<u8>) -> Self {
        self.answers = vec![answer];
        self
    }
}
impl Hid for Fake {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        Ok(self.answers[0].clone())
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

/// Version answer: buf[3..] = res -> v2.0.5
fn version_answer_v2() -> Vec<u8> {
    let mut a = vec![0u8; 97];
    a[3] = 2;
    a[5] = 5;
    a
}
/// LED count answer: res[2] = 2 devices; pump 24, fan 34.
/// res = answer[3..], so res[2] = answer[5], res[5] = answer[8], res[6] = answer[9].
fn led_count_answer() -> Vec<u8> {
    let mut a = vec![0u8; 97];
    a[5] = 2; // res[2] -> 2 endpoints
    a[8] = 0x18; // res[5] lo of 24
    a[9] = 0x00; // res[6] hi
    a[12] = 0x22; // res[9] lo of 34
    a[13] = 0x00; // res[10] hi
    a
}

fn core_module_with(pid: u16, answer: Vec<u8>) -> (crate::CommanderCore, Fake) {
    let mut m = crate::CommanderCore::default();
    let mut h = Fake::new().with_read_answer(answer);
    m.initialize(&iface(pid), &mut h).unwrap();
    (m, h)
}

#[test]
fn inventory_is_complete() {
    // upstream: 7 detector rows (Core, Core 2, 3..6, XT)
    assert_eq!(crate::devices_table::DETECTORS.len(), 7);
    assert!(crate::table::identify(&iface(0x0c1c)).supported);
    assert!(crate::table::identify(&iface(0x0c2a)).supported);
}

#[test]
fn v2_initialize_sequence_is_byte_exact() {
    let (_m, h) = core_module_with(0x0c1c, version_answer_v2());
    // 1. version query
    assert_eq!(&h.writes[0][0..4], &[0x00, 0x08, 0x02, 0x13]);
    assert_eq!(h.writes[0].len(), 97);
    // 2. SetFanMode: open endpoint + fan payload + close + wake
    assert_eq!(&h.writes[1][0..6], &[0x00, 0x08, 0x0D, 0x00, 0x1E, 0x00]);
    // first data packet: real_len = 15 + 2 = 17
    assert_eq!(
        &h.writes[2][0..8],
        &[0x00, 0x08, 0x06, 0x00, 0x11, 0x00, 0x00, 0x00]
    );
    assert_eq!(&h.writes[2][8..10], &[0x0D, 0x00]);
    assert_eq!(&h.writes[2][10..13], &[0x07, 0x01, 0x08]);
    assert_eq!(h.writes[2].len(), 97);
    // close endpoint
    assert_eq!(&h.writes[3][0..4], &[0x00, 0x08, 0x05, 0x01]);
    // wake
    assert_eq!(&h.writes[4][0..6], &[0x00, 0x08, 0x01, 0x03, 0x00, 0x02]);
    // 3. LED count read: open, read, close
    assert_eq!(&h.writes[5][0..6], &[0x00, 0x08, 0x0D, 0x00, 0x20, 0x00]);
    assert_eq!(&h.writes[6][0..4], &[0x00, 0x08, 0x08, 0x00]);
    assert_eq!(&h.writes[7][0..4], &[0x00, 0x08, 0x05, 0x01]);
    assert_eq!(h.writes.len(), 8);
}

#[test]
fn v1_packet_size_on_version_one() {
    // Commander Core with FW major version 1 switches to the 1025-byte packet
    let mut a = vec![0u8; 1025];
    a[3] = 1; // res[0] -> major version 1
    a[5] = 1; // res[2] -> 1 endpoint (pump)
    a[8] = 0x18; // res[5..7] -> pump 24 LEDs
    let (_m, h) = core_module_with(0x0c1c, a);
    assert_eq!(h.writes[0].len(), 97); // probe uses V2 size first
    assert_eq!(h.writes[1].len(), 1025);
}

#[test]
fn core2_uses_v3_and_xt_uses_385() {
    let (_m, h) = core_module_with(0x0c32, version_answer_v2());
    assert_eq!(h.writes[0].len(), 65);
    let (_m, h) = core_module_with(0x0c2a, version_answer_v2());
    assert_eq!(h.writes[0].len(), 385);
}

#[test]
fn direct_color_frame_is_byte_exact() {
    // pump 24 LEDs + fan header 1 with 34 -> colors 58; V2 split: first chunk 87
    let counts_answer = {
        let mut a = vec![0u8; 97];
        a[5] = 1; // 1 endpoint (pump, 24)
        a[8] = 0x18;
        a
    };
    let (mut m, mut h) = core_module_with(0x0c1c, counts_answer);
    let pump_leds = 24usize;
    // configure one fan zone with 34 LEDs via host zones
    let zones = [
        freergb_module_api::Zone {
            name: "Pump".into(),
            leds: pump_leds,
        },
        freergb_module_api::Zone {
            name: "Corsair Fan Header 1".into(),
            leds: 34,
        },
    ];
    m.initialize_with_zones(&iface(0x0c1c), Some(&zones), &mut h)
        .unwrap();
    let colors: Vec<Rgb> = (0..58)
        .map(|i| Rgb {
            r: i as u8,
            g: (i * 2) as u8,
            b: (i * 3) as u8,
        })
        .collect();
    let writes_before = h.writes.len();
    m.frame(&colors, &mut h).unwrap();
    // WriteData: open endpoint command frame
    assert_eq!(
        &h.writes[writes_before][0..6],
        &[0x00, 0x08, 0x0D, 0x00, 0x22, 0x00]
    );
    // first data packet: upstream pads the 5 empty fan zones too, so the
    // RGB buffer offset is pump 24*3 + fan 34*3 + 5*(34*3 padding) = 684;
    // real_len = 684 + 2 = 686 = 0x02AE
    let d = &h.writes[writes_before + 1];
    assert_eq!(&d[0..4], &[0x00, 0x08, 0x06, 0x00]);
    assert_eq!(&d[4..6], &[0xAE, 0x02]); // real_len little endian
    assert_eq!(&d[6..8], &[0x00, 0x00]);
    // data-type bytes inside the packet header, then the LED colors
    assert_eq!(&d[8..10], &[0x12, 0x00]);
    assert_eq!(&d[10..13], &[0, 0, 0]);
    assert_eq!(&d[10 + 66..10 + 69], &[22, 44, 66]); // color 22 of pump
                                                     // 684 data bytes -> first chunk 87, then 93-byte chunks
    let cont = &h.writes[writes_before + 2];
    assert_eq!(&cont[0..4], &[0x00, 0x08, 0x07, 0x00]);
    // close endpoint command comes after: 1 first packet + 7 continuations
    // (684-87 = 597 = 6*93 + 39)
    assert_eq!(
        &h.writes[writes_before + 9][0..4],
        &[0x00, 0x08, 0x05, 0x01]
    );
    m.shutdown(None, &mut h).unwrap();
    // shutdown sends the hardware mode command [01 03] [00 01]
    let last = h.writes.last().unwrap();
    assert_eq!(&last[0..6], &[0x00, 0x08, 0x01, 0x03, 0x00, 0x01]);
}

#[test]
fn frame_length_is_checked_and_layouts_valid() {
    let (mut m, mut h) = core_module_with(0x0c1c, led_count_answer());
    // zones: Pump 24 + 6 fan headers -> host may add fan LEDs
    let zones: Vec<freergb_module_api::Zone> = (0..7)
        .map(|i| freergb_module_api::Zone {
            name: format!("Z{}", i),
            leds: if i == 0 { 24 } else { 34 },
        })
        .collect();
    let caps = m
        .initialize_with_zones(&iface(0x0c1c), Some(&zones), &mut h)
        .unwrap();
    assert_eq!(
        caps.zones.iter().map(|z| z.leds).sum::<usize>(),
        24 + 6 * 34
    );
    let total = 24 + 6 * 34;
    let colors: Vec<Rgb> = vec![Rgb::default(); total];
    m.frame(&colors, &mut h).unwrap();
    assert!(m.frame(&colors[..total - 1], &mut h).is_err());
    // valid layout: unique positions, non-empty names, zones sum
    let mut seen = std::collections::HashSet::new();
    for led in &caps.layout.leds {
        assert!(!led.name.is_empty());
        assert!(seen.insert(led.pos));
    }
}
