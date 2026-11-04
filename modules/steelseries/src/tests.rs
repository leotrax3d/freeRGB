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
fn first_detector() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    iface(d.pid)
}
#[test]
fn inventory_is_complete() {
    // upstream: SteelSeriesControllerDetect.cpp registers 76 detector rows
    // on VID 0x1038 across 17 sub-controller callbacks.
    assert_eq!(crate::devices_table::DETECTORS.len(), 76)
}
#[test]
fn inventoried_devices_are_not_claimed_before_protocol_port() {
    let result = crate::table::identify(&first_detector());
    assert!(!result.supported);
    assert!(result.reason.unwrap().contains("protocol is not ported"));
}
struct Fake {
    writes: Vec<Vec<u8>>,
    features: Vec<Vec<u8>>,
}
impl Fake {
    fn new() -> Self {
        Fake {
            writes: Vec::new(),
            features: Vec::new(),
        }
    }
}
impl Hid for Fake {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
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
fn unported_steelseries_devices_stay_unsupported() {
    // DetectSteelSeriesAerox3 / Sensei / Arctis / QCK / Apex handlers are not
    // ported yet and must never report supported.
    for handler in [
        "DetectSteelSeriesAerox3",
        "DetectSteelSeriesAerox3Wireless",
        "DetectSteelSeriesSensei",
        "DetectSteelSeriesRival5",
        "DetectSteelSeriesSiberia350",
        "DetectSteelSeriesArctis5",
        "DetectSteelSeriesQCKPrism",
        "DetectSteelSeriesApex",
    ] {
        if let Some(d) = crate::devices_table::DETECTORS
            .iter()
            .find(|d| d.handler == handler)
        {
            let r = crate::table::identify(&iface(d.pid));
            assert!(!r.supported, "{} must stay unsupported", handler);
        }
    }
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
    // Aerox 3 wired is inventoried but its protocol is not ported
    let i = iface(pid_of("Aerox 3 Wired"));
    assert!(m.initialize(&i, &mut h).is_err());
    assert!(h.writes.is_empty() && h.features.is_empty());
    // Rival 3 Wireless uses SteelSeriesAeroxWirelessController upstream and is
    // covered by a later port step; not supported by the Rival3 path yet
    let i = iface(pid_of("Rival 3 Wireless"));
    assert!(m.initialize(&i, &mut h).is_err());
}
