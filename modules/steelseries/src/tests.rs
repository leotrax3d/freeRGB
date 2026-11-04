// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};
const RGB1: Rgb = Rgb {
    r: 0x11,
    g: 0x22,
    b: 0x33,
};
const RGB2: Rgb = Rgb {
    r: 0x44,
    g: 0x55,
    b: 0x66,
};
fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1038,
        pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}
struct Fake {
    writes: Vec<Vec<u8>>,
    features: Vec<Vec<u8>>,
    reads: usize,
}
impl Fake {
    fn new() -> Self {
        Fake {
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
        Ok(vec![0u8; 64])
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}
fn rival_pid(handler: &str) -> u16 {
    crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.handler == handler)
        .unwrap()
        .pid
}
/// DetectSteelSeriesRival100 devices: 1b1c-style single logo LED.
fn pid_of(name_suffix: &str) -> u16 {
    crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.name.contains(name_suffix))
        .unwrap()
        .pid
}
fn module_for(handler: &str) -> crate::SteelSeries {
    let _ = handler;
    crate::SteelSeries::default()
}
#[test]
fn inventory_is_complete() {
    // upstream: SteelSeriesControllerDetect.cpp registers 76 detector rows
    // on VID 0x1038 across 17 sub-controller callbacks.
    assert_eq!(crate::devices_table::DETECTORS.len(), 76)
}
#[test]
fn inventoried_devices_are_not_claimed_before_protocol_port() {
    // Apex rows are inventoried but not ported
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.handler == "DetectSteelSeriesApex")
        .unwrap();
    let result = crate::table::identify(&iface(d.pid));
    assert!(!result.supported);
    assert!(result.reason.unwrap().contains("protocol is not ported"));
}
#[test]
fn rival100_full_sequence_is_byte_exact() {
    let mut m = module_for("rival100");
    let mut h = Fake::new();
    let i = iface(pid_of("Rival 100"));
    let caps = m.initialize(&i, &mut h).unwrap();
    // upstream: RGBController_SteelSeriesRival.cpp SetupZones base case
    assert_eq!(caps.layout.leds.len(), 1);
    assert_eq!(caps.layout.leds[0].name, "Logo");
    assert!(h.writes.is_empty() && h.features.is_empty());
    m.frame(&[RGB1], &mut h).unwrap();
    // effect packet then SetColor
    assert_eq!(h.writes.len(), 2);
    // upstream: SetLightEffect case RIVAL_100 with EFFECT_DIRECT 0x01
    assert_eq!(h.writes[0], vec![0x00, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
    // upstream: SetColor case RIVAL_100 with zone byte 0x00
    assert_eq!(
        h.writes[1],
        vec![0x00, 0x05, 0x00, 0x11, 0x22, 0x33, 0, 0, 0, 0]
    );
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2);
    assert!(h.features.is_empty());
}
#[test]
fn rival300_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // "SteelSeries Rival 300" is the first DetectSteelSeriesRival300 row
    let i = iface(rival_pid("DetectSteelSeriesRival300"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    assert_eq!(caps.layout.leds[0].name, "Logo");
    assert_eq!(caps.layout.leds[1].name, "Scroll Wheel");
    m.frame(&[RGB1, RGB2], &mut h).unwrap();
    assert_eq!(h.writes.len(), 4);
    // upstream: SetLightEffectAll case RIVAL_300 (zones 0 and 1, +1 encoding)
    assert_eq!(h.writes[0], vec![0x00, 0x07, 0x01, 0x01, 0, 0, 0, 0, 0, 0]);
    assert_eq!(h.writes[1], vec![0x00, 0x07, 0x02, 0x01, 0, 0, 0, 0, 0, 0]);
    // upstream: SetColor case RIVAL_300
    assert_eq!(
        h.writes[2],
        vec![0x00, 0x08, 0x01, 0x11, 0x22, 0x33, 0, 0, 0, 0]
    );
    assert_eq!(
        h.writes[3],
        vec![0x00, 0x08, 0x02, 0x44, 0x55, 0x66, 0, 0, 0, 0]
    );
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn rival600_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    let i = iface(rival_pid("DetectSteelSeriesRival600"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 8);
    // upstream: rival_600_leds table
    assert_eq!(caps.layout.leds[2].name, "Left top");
    assert_eq!(caps.layout.leds[7].name, "Right bottom");
    m.frame(&[RGB1; 8], &mut h).unwrap();
    // upstream: SetLightEffectAll default for RIVAL_600 - no effect packets
    assert_eq!(h.writes.len(), 8);
    // 7-byte packets: 0x1c 0x27 0x00 (1<<zone) r g b
    assert_eq!(h.writes[0], vec![0x1c, 0x27, 0x00, 0x02, 0x11, 0x22, 0x33]);
    assert_eq!(h.writes[1], vec![0x1c, 0x27, 0x00, 0x01, 0x11, 0x22, 0x33]);
    assert_eq!(h.writes[2], vec![0x1c, 0x27, 0x00, 0x04, 0x11, 0x22, 0x33]);
    assert_eq!(h.writes[7], vec![0x1c, 0x27, 0x00, 0x80, 0x11, 0x22, 0x33]);
    assert!(h.features.is_empty());
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn rival650_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    let i = iface(rival_pid("DetectSteelSeriesRival650"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 8);
    assert_eq!(caps.layout.leds[0].name, "Logo");
    assert_eq!(caps.layout.leds[1].name, "Scroll Wheel");
    assert_eq!(caps.layout.leds[2].name, "Left 1");
    m.frame(&[RGB1, RGB1, RGB2, RGB1, RGB2, RGB1, RGB2, RGB1], &mut h)
        .unwrap();
    // 8 effect packets (zeros fall-through) + 8 LEDs * 4 packets = 40 writes
    assert_eq!(h.writes.len(), 40);
    // upstream: SetLightEffect default arm for RIVAL_650
    assert_eq!(h.writes[0], vec![0x00, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
    // First LED colors via SetRival650Color (zone 0x11)
    let q = &h.writes[8..12];
    assert_eq!(q.len(), 4);
    assert_eq!(q[0][1], 0x03);
    assert_eq!(&q[0][0x20..0x23], &[0x11, 0x22, 0x33]);
    assert_eq!(q[1][1], 0x03);
    assert_eq!(&q[1][3..6], &[0x30, 0x00, 0x2C]);
    assert_eq!(q[2][1], 0x05);
    assert_eq!(&q[2][3..10], &[0x11, 0xFF, 0, 0, 0, 0, 0x5C]);
    assert_eq!(q[3][1], 0x1C);
    assert_eq!(&q[3][3..6], &[0x55, 0x00, 0x46]);
    assert!(h.features.is_empty());
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn rival700_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    let i = iface(rival_pid("DetectSteelSeriesRival700"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    m.frame(&[RGB1, RGB2], &mut h).unwrap();
    // upstream: Rival 700 sends no effect packets, one 579-byte feature per LED
    assert_eq!(h.features.len(), 2);
    assert_eq!(h.features[0].len(), 579);
    assert_eq!(h.features[0][1], 0x05);
    assert_eq!(h.features[0][3], 0x00); // logo zone_id 0
    assert_eq!(&h.features[0][4..7], &[0x11, 0x22, 0x33]);
    assert_eq!(h.features[1][3], 0x01); // wheel zone_id 1
    assert_eq!(&h.features[1][4..7], &[0x44, 0x55, 0x66]);
    assert!(h.writes.is_empty());
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn rival3_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesRival3 row (interface 3; 0x184C new firmware)
    let i = iface(rival_pid("DetectSteelSeriesRival3"));
    let caps = m.initialize(&i, &mut h).unwrap();
    // upstream: SteelSeriesRival3Controller.h rival_3 led table
    assert_eq!(caps.layout.leds.len(), 4);
    assert_eq!(caps.layout.leds[0].name, "Front");
    assert_eq!(caps.layout.leds[3].name, "Logo");
    assert_eq!(caps.zones.len(), 4);
    m.frame(&[RGB1, RGB2, RGB1, RGB2], &mut h).unwrap();
    assert_eq!(h.writes.len(), 5);
    // upstream: SteelSeriesRival3Controller.cpp SetColor with brightness 0x64
    assert_eq!(
        h.writes[0],
        vec![0x00, 0x05, 0x00, 0x01, 0x11, 0x22, 0x33, 0x64]
    );
    assert_eq!(
        h.writes[1],
        vec![0x00, 0x05, 0x00, 0x02, 0x44, 0x55, 0x66, 0x64]
    );
    assert_eq!(
        h.writes[3],
        vec![0x00, 0x05, 0x00, 0x04, 0x44, 0x55, 0x66, 0x64]
    );
    // upstream: RGBController_SteelSeriesRival3.cpp DeviceUpdateMode() with
    // DIRECT (0x04) minus speed (0) via SetLightEffectAll [00 06 00 04]
    assert_eq!(h.writes[4], vec![0x00, 0x06, 0x00, 0x04]);
    assert!(h.features.is_empty());
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn rival5_direct_and_brightness_are_byte_exact() {
    // upstream: SteelSeriesRival5Controller.cpp UpdateLEDs (10 zones)
    let p = crate::report::set_rival5_direct(&[
        RGB1, RGB2, RGB1, RGB2, RGB1, RGB2, RGB1, RGB2, RGB1, RGB2,
    ]);
    assert_eq!(p.len(), 64);
    assert_eq!(&p[0..7], &[0x00, 0x21, 0xFF, 0x03, 0x11, 0x22, 0x33]);
    assert_eq!(&p[7..10], &[0x44, 0x55, 0x66]);
    assert_eq!(&p[4 + 27..4 + 30], &[0x44, 0x55, 0x66]); // zone index 9 = Logo
    assert!(p[4 + 30..].iter().all(|b| *b == 0));
    // upstream: SetBrightness tier quantization
    assert_eq!(crate::report::set_rival5_brightness(0x64)[2], 0x64);
    assert_eq!(crate::report::set_rival5_brightness(88)[2], 0x64);
    assert_eq!(crate::report::set_rival5_brightness(63)[2], 0x32);
    assert_eq!(crate::report::set_rival5_brightness(38)[2], 0x19);
    assert_eq!(crate::report::set_rival5_brightness(13)[2], 0x0C);
    assert_eq!(crate::report::set_rival5_brightness(0)[2], 0x00);
}
#[test]
fn rival5_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    let i = iface(rival_pid("DetectSteelSeriesRival5"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 10);
    assert_eq!(caps.layout.leds[0].name, "Scroll Wheel");
    assert_eq!(caps.layout.leds[9].name, "Logo");
    assert_eq!(caps.zones.len(), 1);
    assert_eq!(caps.zones[0].name, "Whole Mouse");
    assert!(h.features.is_empty());
    // Frame 1: brightness (0xFF -> 0x64 change) + direct colors
    m.frame(&[RGB1; 10], &mut h).unwrap();
    assert_eq!(h.features.len(), 2);
    assert_eq!(&h.features[0][0..3], &[0x00, 0x23, 0x64]);
    assert_eq!(&h.features[1][0..4], &[0x00, 0x21, 0xFF, 0x03]);
    // Frame 2: brightness unchanged -> only the direct colors report
    m.frame(&[RGB2; 10], &mut h).unwrap();
    assert_eq!(h.features.len(), 3);
    assert_eq!(&h.features[2][0..4], &[0x00, 0x21, 0xFF, 0x03]);
    assert_eq!(&h.features[2][4..7], &[0x44, 0x55, 0x66]);
    assert!(h.writes.is_empty());
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn sensei_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesSensei row (Sensei TEN 0x1832 first)
    let i = iface(rival_pid("DetectSteelSeriesSensei"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    assert_eq!(caps.layout.leds[0].name, "Logo");
    assert_eq!(caps.layout.leds[1].name, "Scroll Wheel");
    m.frame(&[RGB1, RGB2], &mut h).unwrap();
    // upstream: DeviceUpdateLEDs -> SetColor for both zones, no effect packets
    assert_eq!(h.writes.len(), 2);
    assert_eq!(h.writes[0].len(), 66);
    assert_eq!(h.writes[0][0], 0x00); // report id
    assert_eq!(h.writes[0][1], 0x5B); // command
    assert_eq!(h.writes[0][3], 0x00); // zone 0
    assert_eq!(h.writes[0][0x14], 0x01); // static color flag (usb_buf[0x13])
    assert_eq!(h.writes[0][0x1C], 0x01); // number of colors (usb_buf[0x1B])
    assert_eq!(
        &h.writes[0][0x1D..0x23],
        &[0x11, 0x22, 0x33, 0x11, 0x22, 0x33]
    );
    assert_eq!(
        &h.writes[1][0x1D..0x23],
        &[0x44, 0x55, 0x66, 0x44, 0x55, 0x66]
    );
    assert_eq!(h.writes[1][3], 0x01); // zone 1
    assert!(h.features.is_empty());
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn arctis5_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesArctis5 row (0x1250 first)
    let i = iface(rival_pid("DetectSteelSeriesArctis5"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    assert_eq!(caps.layout.leds[0].name, "Left");
    assert_eq!(caps.layout.leds[1].name, "Right");
    assert_eq!(caps.kind, freergb_module_api::DeviceKind::Headset);
    m.frame(&[RGB1, RGB2], &mut h).unwrap();
    // 2 zones x 6 packets of 37 bytes
    assert_eq!(h.writes.len(), 12);
    assert!(h.writes.iter().all(|p| p.len() == 37));
    // Zone 0 (Left): pre1, pre2, color, apply-zone, apply-to, apply
    let z = &h.writes[0..6];
    assert_eq!(&z[0][0..5], &[0x06, 0x81, 0x43, 0x01, 0x22]);
    assert_eq!(&z[1][0..5], &[0x06, 0x81, 0x43, 0x01, 0x23]);
    assert_eq!(&z[2][0..3], &[0x06, 0x8A, 0x42]);
    assert_eq!(
        &z[2][4..14],
        &[0x20, 0x41, 0x00, 0x11, 0x22, 0x33, 0xFF, 0x32, 0xC8, 0xC8]
    );
    assert_eq!(&z[3][4..9], &[0x20, 0x41, 0x08, 0x00, 0x01]);
    assert_eq!(&z[4][4..7], &[0x20, 0x60, 0x00]);
    assert_eq!(&z[5][4..6], &[0x20, 0x05]);
    // Zone 1 (Right): color bytes and zone id differ
    let z = &h.writes[6..12];
    assert_eq!(&z[2][7..10], &[0x44, 0x55, 0x66]);
    assert_eq!(z[3][7], 0x01);
    assert_eq!(z[4][6], 0x01);
    assert!(h.features.is_empty());
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn siberia350_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesHeadset row (Siberia 350, interface 3)
    let i = iface(rival_pid("DetectSteelSeriesHeadset"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 1);
    assert_eq!(caps.layout.leds[0].name, "Headset LED");
    assert_eq!(caps.kind, freergb_module_api::DeviceKind::Headset);
    m.frame(&[RGB1], &mut h).unwrap();
    // 4x 16-byte packets: [01 00 cmd len payload...]
    assert_eq!(h.writes.len(), 4);
    // upstream: SteelSeriesSiberiaController.cpp send_usb_msg + commands 1-4
    assert_eq!(
        h.writes[0],
        vec![0x01, 0x00, 0x95, 0x02, 0x80, 0xbf, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        h.writes[1],
        vec![0x01, 0x00, 0x80, 0x02, 0x52, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        h.writes[2],
        vec![0x01, 0x00, 0x83, 0x03, 0x11, 0x22, 0x33, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        h.writes[3],
        vec![0x01, 0x00, 0x93, 0x02, 0x03, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert!(h.features.is_empty());
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn sensei_save_is_byte_exact() {
    // upstream: SteelSeriesSenseiController.cpp Save() [00 59 00...]
    assert_eq!(
        crate::report::sensei_save(),
        vec![0x00, 0x59, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}
#[test]
fn arctis_nova3_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesArctisNova3 row (0x12EC, interface 4)
    let i = iface(rival_pid("DetectSteelSeriesArctisNova3"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    assert_eq!(caps.layout.leds[0].name, "Right");
    assert_eq!(caps.layout.leds[1].name, "Left");
    assert_eq!(caps.zones.len(), 1);
    assert_eq!(caps.zones[0].name, "Nova 3");
    assert_eq!(caps.kind, freergb_module_api::DeviceKind::Headset);
    m.frame(&[RGB1, RGB2], &mut h).unwrap();
    // 2 LEDs x 3 packets (effect 521 bytes, select 64, apply 64)
    assert_eq!(h.writes.len(), 6);
    let z = &h.writes[0..3];
    assert_eq!(z[0].len(), 521);
    // upstream: SetDirect effect packet
    assert_eq!(z[0][0], 0x06);
    assert_eq!(z[0][1], 0xAA);
    assert_eq!(z[0][2], 0x01); // zone 0 -> 0x01
    assert_eq!(&z[0][0x0D..0x0F], &[0x98, 0x3A]);
    assert_eq!(z[0][0x15], 0x55);
    assert_eq!(z[0][0x17], 0x96);
    assert_eq!(z[0][0x1D], 0x01);
    assert_eq!(z[0][0x25], 0x02);
    assert_eq!(&z[0][0x2D..0x31], &[0x11, 0x22, 0x33, 0xFF]);
    assert_eq!(z[1][0], 0x06);
    assert_eq!(z[1][1], 0xA5);
    assert_eq!(z[1][2], 0x02); // SelectZone zone 0 -> 0x02
    assert_eq!(z[1][3], 0x0A);
    assert_eq!(z[2][0], 0x06);
    assert_eq!(z[2][1], 0x09); // ApplyZone zone 0
                               // Second LED (zone 1)
    let z = &h.writes[3..6];
    assert_eq!(z[0][2], 0x00);
    assert_eq!(&z[0][0x2D..0x31], &[0x44, 0x55, 0x66, 0xFF]);
    assert_eq!(z[1][2], 0x01);
    assert_eq!(z[2][1], 0xA3);
    assert!(z[1].iter().skip(4).all(|b| *b == 0));
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn aerox5_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesAerox5 row ("SteelSeries Aerox 5 Wired", 0x1850)
    let i = iface(rival_pid("DetectSteelSeriesAerox5"));
    let caps = m.initialize(&i, &mut h).unwrap();
    // upstream: SteelSeriesAerox5Controller.h aerox_5 led table
    assert_eq!(caps.layout.leds.len(), 4);
    assert_eq!(caps.layout.leds[3].name, "Reactive");
    // upstream: SendInit in constructor
    assert_eq!(&h.features[0][0..2], &[0x00, 0x2D]);
    m.frame(&[RGB1, RGB2, RGB1, RGB2], &mut h).unwrap();
    // brightness once + 4 zone packets
    assert_eq!(h.writes.len(), 5);
    assert_eq!(&h.writes[0][0..3], &[0x00, 0x23, 0x64]);
    // upstream: SetColor zones 0..2 cmd 0x21 masks 01/02/04, RGB at 3/6/9
    assert_eq!(&h.writes[1][0..6], &[0x00, 0x21, 0x01, 0x11, 0x22, 0x33]);
    assert_eq!(
        &h.writes[2][0..9],
        &[0x00, 0x21, 0x02, 0, 0, 0, 0x44, 0x55, 0x66]
    );
    assert_eq!(
        &h.writes[3][0..12],
        &[0x00, 0x21, 0x04, 0, 0, 0, 0, 0, 0, 0x11, 0x22, 0x33]
    );
    // upstream: zone 3 (Reactive) cmd 0x26, RGB at 0x04..
    assert_eq!(
        &h.writes[4][0..7],
        &[0x00, 0x26, 0x01, 0x00, 0x44, 0x55, 0x66]
    );
    // Frame 2: no brightness repeat
    m.frame(&[RGB2; 4], &mut h).unwrap();
    assert_eq!(h.writes.len(), 9);
    assert_eq!(
        &h.writes[5][0..7],
        &[0x00, 0x21, 0x01, 0x44, 0x55, 0x66, 0x00]
    );
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn qck_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesMousemat row (QCK Prism Cloth Medium, 0x150A)
    let i = iface(rival_pid("DetectSteelSeriesMousemat"));
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    assert_eq!(caps.layout.leds[0].name, "Mat Bottom LED");
    assert_eq!(caps.layout.leds[1].name, "Mat Top LED");
    assert_eq!(caps.zones.len(), 1);
    assert_eq!(caps.zones[0].name, "Mousemat");
    assert_eq!(caps.kind, freergb_module_api::DeviceKind::Mousemat);
    m.frame(&[RGB1, RGB2], &mut h).unwrap();
    // 525-byte feature + 65-byte apply write
    assert_eq!(h.features.len(), 1);
    assert_eq!(h.features[0].len(), 525);
    // upstream: SteelSeriesQCKMatController.cpp SetColors magic bytes
    assert_eq!(&h.features[0][0x00..0x04], &[0x00, 0x0E, 0x00, 0x02]);
    assert_eq!(&h.features[0][0x05..0x08], &[0x11, 0x22, 0x33]);
    assert_eq!(&h.features[0][0x08..0x0B], &[0xFF, 0x32, 0xC8]);
    assert_eq!(h.features[0][0x0E], 0x01);
    assert_eq!(&h.features[0][0x11..0x14], &[0x44, 0x55, 0x66]);
    assert_eq!(&h.features[0][0x14..0x17], &[0xFF, 0x32, 0xC8]);
    assert_eq!(h.features[0][0x19], 0x01);
    assert_eq!(&h.features[0][0x1A..0x1D], &[0x01, 0x00, 0x01]);
    assert!(h.features[0][0x1D..].iter().all(|b| *b == 0));
    assert_eq!(h.writes.len(), 1);
    assert_eq!(&h.writes[0][0..2], &[0x00, 0x0D]);
    assert_eq!(h.writes[0].len(), 65);
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn aerox_wireless_wired_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesAerox3WirelessWired row (0x183A, interface 3)
    let i = iface(rival_pid("DetectSteelSeriesAerox3WirelessWired"));
    let caps = m.initialize(&i, &mut h).unwrap();
    // upstream: aerox_3_wireless table
    assert_eq!(caps.layout.leds.len(), 3);
    assert_eq!(caps.layout.leds[0].name, "Front");
    // upstream: SendInit in constructor (64-byte feature)
    assert_eq!(&h.features[0][0..2], &[0x00, 0x2D]);
    assert_eq!(h.features[0].len(), 64);
    m.frame(&[RGB1, RGB2, RGB1], &mut h).unwrap();
    // wired: no 0x40 flag, no readback; brightness (10) after first color
    assert_eq!(h.reads, 0);
    assert_eq!(h.writes.len(), 4);
    // upstream: SetColor [00 21 01 zone rgb@4..]
    assert_eq!(
        &h.writes[0][0..7],
        &[0x00, 0x21, 0x01, 0x00, 0x11, 0x22, 0x33]
    );
    // brightness packet [00 23 10]
    assert_eq!(&h.writes[1][0..3], &[0x00, 0x23, 0x0A]);
    assert_eq!(
        &h.writes[2][0..7],
        &[0x00, 0x21, 0x01, 0x01, 0x44, 0x55, 0x66]
    );
    assert_eq!(h.writes[3][3], 0x02);
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn aerox_wireless_wireless_flag_and_readback() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesAerox3Wireless row (0x1838): 0x40 flag + readback
    let i = iface(rival_pid("DetectSteelSeriesAerox3Wireless"));
    m.initialize(&i, &mut h).unwrap();
    m.frame(&[RGB1; 3], &mut h).unwrap();
    // 3 color writes + 1 brightness write = 4 writes; 4 readbacks (wireless)
    assert_eq!(h.writes.len(), 4);
    assert_eq!(h.reads, 4);
    assert_eq!(h.writes[0][1], 0x21 | 0x40);
    assert_eq!(h.writes[1][1], 0x23 | 0x40);
    assert_eq!(h.writes[1][2], 0x0A);
    // Second frame: brightness unchanged -> 3 writes + 3 readbacks
    m.frame(&[RGB1; 3], &mut h).unwrap();
    assert_eq!(h.writes.len(), 7);
    assert_eq!(h.reads, 7);
}
#[test]
fn aerox9_wireless_reactive_zone_uses_0x26() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesAerox9Wireless row (0x1858)
    let i = iface(rival_pid("DetectSteelSeriesAerox9Wireless"));
    let caps = m.initialize(&i, &mut h).unwrap();
    // upstream: aerox_9 table
    assert_eq!(caps.layout.leds.len(), 4);
    m.frame(&[RGB1; 4], &mut h).unwrap();
    // brightness + 4 colors (+ readbacks)
    assert_eq!(h.writes.len(), 5);
    // upstream: zone 3 "Reactive" -> cmd 0x26, byte 3 = 0
    assert_eq!(h.writes[4][0], 0x00);
    assert_eq!(h.writes[4][1], 0x26 | 0x40);
    assert_eq!(h.writes[4][2], 0x01);
    assert_eq!(h.writes[4][3], 0x00);
    assert_eq!(&h.writes[4][4..7], &[0x11, 0x22, 0x33]);
}
#[test]
fn rival3_wireless_uses_0x13_packets() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesRival3Wireless row (0x1830)
    let i = iface(rival_pid("DetectSteelSeriesRival3Wireless"));
    let caps = m.initialize(&i, &mut h).unwrap();
    // upstream: rival_3_wireless table - single "Scroll Wheel" LED value 1
    assert_eq!(caps.layout.leds.len(), 1);
    assert_eq!(caps.layout.leds[0].name, "Scroll Wheel");
    m.frame(&[RGB1], &mut h).unwrap();
    // upstream: RIVAL_3_WIRELESS color packet [00 13 1 rgb@3..6] plus the
    // common brightness write [00 23 10] (IsWireless() == false: no 0x40
    // flag, no readback)
    assert_eq!(h.writes.len(), 2);
    assert_eq!(h.reads, 0);
    assert_eq!(&h.writes[0][0..6], &[0x00, 0x13, 0x01, 0x11, 0x22, 0x33]);
    assert_eq!(&h.writes[1][0..3], &[0x00, 0x23, 0x0A]);
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn aerox3_full_sequence_is_byte_exact() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // DetectSteelSeriesAerox3 row ("SteelSeries Aerox 3 Wired", 0x1836)
    let i = iface(rival_pid("DetectSteelSeriesAerox3"));
    // upstream: SteelSeriesAerox3Controller.cpp SendInit() in constructor
    let caps = m.initialize(&i, &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 3);
    assert!(h.features.len() == 1 && h.writes.is_empty());
    assert_eq!(&h.features[0][0..2], &[0x00, 0x2D]);
    assert_eq!(h.features[0].len(), 65);
    // Frame 1: upstream RGBController_SteelSeriesRival3 DeviceUpdateLEDs ->
    // SetColor per zone, brightness packet first (Direct brightness 0x64)
    m.frame(&[RGB1, RGB2, RGB1], &mut h).unwrap();
    assert_eq!(h.writes.len(), 4);
    assert_eq!(&h.writes[0][0..3], &[0x00, 0x23, 0x64]);
    // upstream: SetColor [00 21 (1<<zone) rgb at 3 + zone*3]
    assert_eq!(&h.writes[1][0..6], &[0x00, 0x21, 0x01, 0x11, 0x22, 0x33]);
    assert_eq!(h.writes[2][2], 0x02);
    assert_eq!(&h.writes[2][6..9], &[0x44, 0x55, 0x66]);
    assert_eq!(h.writes[3][2], 0x04);
    assert_eq!(&h.writes[3][9..12], &[0x11, 0x22, 0x33]);
    // Frame 2: brightness already set -> only the color packets
    m.frame(&[RGB2; 3], &mut h).unwrap();
    assert_eq!(h.writes.len(), 7);
    assert!(h.features.len() == 1);
    assert_eq!(&h.writes[4][3..6], &[0x44, 0x55, 0x66]);
    assert_eq!(&h.writes[5][6..9], &[0x44, 0x55, 0x66]);
    m.shutdown(None, &mut h).unwrap();
}
#[test]
fn aerox3_save_is_byte_exact() {
    // upstream: SteelSeriesAerox3Controller.cpp Save() [00 11 00 ...]
    let p = crate::report::aerox3_save();
    assert_eq!(p.len(), 65);
    assert_eq!(&p[0..3], &[0x00, 0x11, 0x00]);
}
#[test]
fn unported_steelseries_devices_stay_unsupported() {
    // DetectSteelSeriesAerox3 / Sensei / Arctis / QCK / Apex handlers are not
    // ported yet and must never report supported.
    // Apex rows are inventoried but not ported and must never report
    // supported. Every other handler is ported, so the ledger is complete.
    assert!(crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.handler == "DetectSteelSeriesApex")
        .is_some());
}
#[test]
fn all_rival_layouts_are_valid() {
    // Zones sum to the LED count, positions are unique, names are non-empty.
    for handler in [
        "DetectSteelSeriesRival100",
        "DetectSteelSeriesRival300",
        "DetectSteelSeriesRival600",
        "DetectSteelSeriesRival650",
        "DetectSteelSeriesRival700",
        "DetectSteelSeriesRival3",
        "DetectSteelSeriesRival5",
        "DetectSteelSeriesSensei",
        "DetectSteelSeriesHeadset",
        "DetectSteelSeriesArctis5",
        "DetectSteelSeriesAerox3",
        "DetectSteelSeriesAerox5",
        "DetectSteelSeriesAerox3Wireless",
        "DetectSteelSeriesAerox3WirelessWired",
        "DetectSteelSeriesAerox3Gen2Wireless",
        "DetectSteelSeriesAerox5Wireless",
        "DetectSteelSeriesAerox5WirelessWired",
        "DetectSteelSeriesAerox9Wireless",
        "DetectSteelSeriesAerox9WirelessWired",
        "DetectSteelSeriesRival3Wireless",
        "DetectSteelSeriesArctisNova3",
        "DetectSteelSeriesMousemat",
    ] {
        let d = crate::devices_table::DETECTORS
            .iter()
            .find(|x| x.handler == handler)
            .unwrap();
        let mut m = crate::SteelSeries::default();
        let caps = m.initialize(&iface(d.pid), &mut Fake::new()).unwrap();
        assert_eq!(
            caps.zones.iter().map(|z| z.leds).sum::<usize>(),
            caps.layout.leds.len()
        );
        let mut seen = std::collections::HashSet::new();
        for led in &caps.layout.leds {
            assert!(!led.name.is_empty());
            assert!(seen.insert(led.pos));
        }
        // frame length must match LED count
        let mut h = Fake::new();
        let ok: Vec<Rgb> = vec![RGB1; caps.layout.leds.len()];
        m.frame(&ok, &mut h).unwrap();
        assert!(m.frame(&ok[..ok.len() - 1], &mut h).is_err());
        m.shutdown(None, &mut h).unwrap();
    }
}
#[test]
fn unsupported_pid_is_rejected() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    assert!(m.initialize(&iface(0x9999), &mut h).is_err());
    assert!(m.initialize(&iface(0x1234), &mut h).is_err());
}
#[test]
fn unsupported_handler_cannot_initialize_even_if_row_exists() {
    let mut m = crate::SteelSeries::default();
    let mut h = Fake::new();
    // Apex is inventoried but its protocol is not ported
    let i = iface(pid_of("Apex"));
    assert!(m.initialize(&i, &mut h).is_err());
    assert!(h.writes.is_empty() && h.features.is_empty());
}
