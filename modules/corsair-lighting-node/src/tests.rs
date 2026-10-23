// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError, Zone};

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 7);
    // upstream: all Lighting Node family detectors share VID 0x1B1C
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1b1c));
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

#[test]
fn packets_are_byte_exact() {
    // upstream: CorsairLightingNodeController.cpp SendPortState/SendDirect/SendCommit
    assert_eq!(&crate::report::state(1)[0..4], &[0, 0x38, 1, 2]);
    assert_eq!(
        &crate::report::direct(1, 12, &[Rgb { r: 1, g: 2, b: 3 }], 2)[0..7],
        &[0, 0x32, 1, 12, 1, 2, 3]
    );
    assert_eq!(&crate::report::commit()[0..3], &[0, 0x33, 0xff]);
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::N::default();
    let mut h = Fake { writes: Vec::new() };
    let caps = m
        .initialize_with_zones(
            &info_for(0),
            Some(&[
                Zone {
                    name: "A".into(),
                    leds: 60,
                },
                Zone {
                    name: "B".into(),
                    leds: 50,
                },
            ]),
            &mut h,
        )
        .unwrap();
    assert_eq!(caps.layout.leds.len(), 110);
    assert_eq!(caps.zones[0].name, "Corsair RGB Header 1");
    assert!(h.writes.is_empty());
    let mut colors = vec![Rgb { r: 1, g: 2, b: 3 }; 60];
    colors.extend(vec![Rgb { r: 4, g: 5, b: 6 }; 50]);
    m.frame(&colors, &mut h).unwrap();
    // channel 0: state + (2 chunks x 3 planes) + commit = 8 writes
    // channel 1: state + (1 chunk x 3 planes) + commit = 5 writes
    assert_eq!(h.writes.len(), 13);
    assert_eq!(h.writes[0][..4], [0, 0x38, 0, 2]);
    // chunk 0 of channel 0: R/G/B at start 0
    assert_eq!(&h.writes[1][..7], &[0, 0x32, 0, 0, 50, 0, 1]);
    assert_eq!(&h.writes[2][..7], &[0, 0x32, 0, 0, 50, 1, 2]);
    assert_eq!(&h.writes[3][..7], &[0, 0x32, 0, 0, 50, 2, 3]);
    // chunk 1 of channel 0: R/G/B at start 50
    assert_eq!(&h.writes[4][..7], &[0, 0x32, 0, 50, 10, 0, 1]);
    assert_eq!(&h.writes[6][..7], &[0, 0x32, 0, 50, 10, 2, 3]);
    assert_eq!(h.writes[7][..3], [0, 0x33, 0xff]); // commit channel 0
    assert_eq!(h.writes[8][..4], [0, 0x38, 1, 2]); // state channel 1
    assert_eq!(&h.writes[9][..7], &[0, 0x32, 1, 0, 50, 0, 4]);
    assert_eq!(h.writes[12][..3], [0, 0x33, 0xff]); // commit channel 1
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 13); // upstream sends no exit packets
}

#[test]
fn unsupported_device_fails_initialize_without_io() {
    let mut m = crate::N::default();
    let mut h = Fake { writes: Vec::new() };
    let mut i = info_for(0);
    i.vid = 0x1234;
    assert!(m.initialize(&i, &mut h).is_err());
    assert!(h.writes.is_empty());
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn zones_are_required_and_layouts_are_valid() {
    let mut m = crate::N::default();
    let mut h = Fake { writes: Vec::new() };
    assert!(m
        .initialize_with_zones(
            &info_for(0),
            Some(&[Zone {
                name: "A".into(),
                leds: 5
            }]),
            &mut h
        )
        .is_err());
    // zone size clamped to upstream leds_max 204
    let caps = m
        .initialize_with_zones(
            &info_for(0),
            Some(&[
                Zone {
                    name: "A".into(),
                    leds: 300,
                },
                Zone {
                    name: "B".into(),
                    leds: 0,
                },
            ]),
            &mut h,
        )
        .unwrap();
    assert_eq!(caps.zones[0].leds, 204);
    let sum: usize = caps.zones.iter().map(|z| z.leds).sum();
    assert_eq!(caps.layout.leds.len(), sum);
}
