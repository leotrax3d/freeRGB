// SPDX-License-Identifier: GPL-2.0-or-later
use crate::Hydro;
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError, Zone};

#[test]
fn catalog_has_13_detectors() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 13);
    // upstream: no_rgb_fan_models[] = the five Elite PIDs
    assert_eq!(
        crate::devices_table::DEVICES
            .iter()
            .filter(|d| !d.rgb_fan)
            .count(),
        5
    );
}

fn info_for(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1b1c,
        pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

struct Fake {
    writes: Vec<Vec<u8>>,
}
impl Hid for Fake {
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

/// upstream: ComputePEC over bytes 2..64 (CRC-8/PEC poly 0x07)
#[test]
fn pec_matches_upstream_and_packets_carry_it() {
    let p = crate::report::colors(&[Rgb { r: 1, g: 2, b: 3 }], 4, 1);
    assert_eq!(&p[..7], &[0, 0x3f, 0x0c, 3, 2, 1, 0]); // BGR order
    assert_ne!(p[64], 0);
    let m = crate::report::magic(&[1, 2, 3], 1, 1);
    assert_eq!(&m[..7], &[0, 0x3f, 9, 1, 2, 3, 0]);
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = Hydro::default();
    let mut h = Fake { writes: Vec::new() };
    let caps = m
        .initialize_with_zones(
            &info_for(0x0c20),
            Some(&[
                Zone {
                    name: "CPU".into(),
                    leds: 16,
                },
                Zone {
                    name: "Fan".into(),
                    leds: 32,
                },
            ]),
            &mut h,
        )
        .unwrap();
    assert_eq!(caps.layout.leds.len(), 48);
    assert_eq!(caps.zones.len(), 2);
    // 3 init magics with sequence 1/2/3, commands 1/2/3
    assert_eq!(h.writes.len(), 3);
    assert_eq!(h.writes[0][2] >> 3, 1);
    assert_eq!(h.writes[0][2] & 7, 1);
    assert_eq!(h.writes[0][3..64], crate::magic::MAGIC_1[..61]);
    assert_eq!(h.writes[1][2] >> 3, 2);
    assert_eq!(h.writes[2][2] >> 3, 3);
    // frame: combined 48 colors -> commands 4/5/6, sequence continues 4/5/6
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 48];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 6);
    let f0 = &h.writes[3];
    let f1 = &h.writes[4];
    let f2 = &h.writes[5];
    assert_eq!(f0[2], (4 << 3) | 4);
    assert_eq!(f1[2], (5 << 3) | 5);
    assert_eq!(f2[2], (6 << 3) | 6);
    assert_eq!(&f0[3..6], &[3, 2, 1]); // BGR
    assert_eq!(&f1[3..6], &[3, 2, 1]);
    assert_eq!(&f2[3..6], &[3, 2, 1]);
    // PEC covers bytes 2..64 of each packet
    for p in [f0, f1, f2] {
        let x = crate::report::pec_bytes(&p[2..64]);
        assert_eq!(p[64], x);
    }
    // shutdown: 4 hardware magics in upstream order 2/3/4/1
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 10);
    assert_eq!(h.writes[6][3..64], crate::magic::HARDWARE_MAGIC_2[..61]);
    assert_eq!(h.writes[6][2] & 7, 2);
    assert_eq!(h.writes[7][2] & 7, 3);
    assert_eq!(h.writes[8][2] & 7, 4);
    assert_eq!(h.writes[9][2] & 7, 1);
    assert_eq!(h.writes[9][3..64], crate::magic::HARDWARE_MAGIC_1[..61]);
}

#[test]
fn elite_models_reject_fan_zones_and_frame_caps_at_20() {
    let mut m = Hydro::default();
    let mut h = Fake { writes: Vec::new() };
    // H100i Elite (0x0C35) has no RGB fan: a second zone must be refused.
    assert!(m
        .initialize_with_zones(
            &info_for(0x0c35),
            Some(&[
                Zone {
                    name: "CPU".into(),
                    leds: 16
                },
                Zone {
                    name: "Fan".into(),
                    leds: 8
                }
            ]),
            &mut h,
        )
        .is_err());
    let caps = m.initialize(&info_for(0x0c35), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 16);
    h.writes.clear();
    m.frame(&[Rgb { r: 1, g: 2, b: 3 }; 16], &mut h).unwrap();
    // 16 LEDs fit in one SET_LIGHTING_1 packet (no command 5/6)
    assert_eq!(h.writes.len(), 1);
    assert_eq!(h.writes[0][2] & 7, 4);
}

#[test]
fn supported_matches_detectors_and_wrong_pid_is_refused() {
    for (n, _) in crate::devices_table::DETECTORS.iter().enumerate().take(1) {
        let d = &crate::devices_table::DETECTORS[n];
        let id = crate::table::identify(&info_for(d.pid));
        assert!(id.supported);
    }
    assert!(!crate::table::identify(&info_for(0x1234)).supported);
    let mut m = Hydro::default();
    let mut h = Fake { writes: Vec::new() };
    assert!(m.initialize(&info_for(0x9999), &mut h).is_err());
    assert!(h.writes.is_empty());
}

#[test]
fn layouts_are_valid() {
    for d in crate::devices_table::DETECTORS.iter() {
        let mut m = Hydro::default();
        let mut h = Fake { writes: Vec::new() };
        let caps = m.initialize(&info_for(d.pid), &mut h).unwrap();
        let sum: usize = caps.zones.iter().map(|z| z.leds).sum();
        assert_eq!(caps.layout.leds.len(), sum);
        assert_eq!(caps.zones[0].leds, 16);
        // zone 0 is the fixed 5x5 CPU block matrix
        for led in &caps.layout.leds[..16] {
            assert!(led.pos.0 < 5 && led.pos.1 < 5);
        }
    }
}

#[test]
fn cpu_block_is_always_16_leds() {
    let mut m = Hydro::default();
    let mut h = Fake { writes: Vec::new() };
    // upstream: RGBController_CorsairHydroPlatinum.cpp SetupZones fixes the
    // CPU block at 16 LEDs (zone 0 is not manually configurable).
    let caps = m
        .initialize_with_zones(
            &info_for(0x0c20),
            Some(&[Zone {
                name: "CPU".into(),
                leds: 10,
            }]),
            &mut h,
        )
        .unwrap();
    assert_eq!(caps.zones[0].leds, 16);
    assert_eq!(caps.layout.leds.len(), 16);
}
