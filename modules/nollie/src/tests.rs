// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn catalog_has_16_detectors() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 16);
    assert_eq!(crate::devices_table::DEVICES.len(), 16);
    // upstream: NollieControllerDetect.cpp registers 3 devices on interface 2.
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.interface == 2)
            .count(),
        3
    );
}

fn iface_for(handler_row: usize) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[handler_row];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: i32::from(d.interface),
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
fn unsupported_handler_is_reported_not_guessed() {
    // upstream: SetChannelLEDs picks SendPacket for the four high-speed PIDs;
    // their channel_index maps and flag-channel writes are not ported.
    let idx = crate::devices_table::DETECTORS
        .iter()
        .position(|d| d.name == "Nollie 32CH")
        .unwrap();
    let id = crate::table::identify(&iface_for(idx));
    assert!(!id.supported);
    assert!(id.reason.as_deref().unwrap().contains("not ported"));
    let idx = crate::devices_table::DETECTORS
        .iter()
        .position(|d| d.name == "Nollie 32_OS2_1")
        .unwrap();
    assert!(!crate::table::identify(&iface_for(idx)).supported);
}

#[test]
fn wrong_interface_is_not_supported() {
    let mut i = iface_for(
        crate::devices_table::DETECTORS
            .iter()
            .position(|d| d.name == "Nollie 8_OS2_1")
            .unwrap(),
    );
    i.interface = 0; // upstream registers this PID on interface 2
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn fixed_device_initialize_frame_shutdown_sequence_is_byte_exact() {
    // upstream: NollieController.cpp SetPacketFS chunking + SendUpdate
    let idx = crate::devices_table::DETECTORS
        .iter()
        .position(|d| d.name == "Nollie 8CH")
        .unwrap();
    let mut m = crate::Nollie::default();
    let mut h = Fake { writes: Vec::new() };
    let caps = m
        .initialize_with_zones(
            &iface_for(idx),
            Some(&[
                freergb_module_api::Zone {
                    name: "A".into(),
                    leds: 24,
                },
                freergb_module_api::Zone {
                    name: "B".into(),
                    leds: 24,
                },
                freergb_module_api::Zone {
                    name: "C".into(),
                    leds: 24,
                },
                freergb_module_api::Zone {
                    name: "D".into(),
                    leds: 24,
                },
                freergb_module_api::Zone {
                    name: "E".into(),
                    leds: 24,
                },
                freergb_module_api::Zone {
                    name: "F".into(),
                    leds: 24,
                },
                freergb_module_api::Zone {
                    name: "G".into(),
                    leds: 24,
                },
                freergb_module_api::Zone {
                    name: "H".into(),
                    leds: 24,
                },
            ]),
            &mut h,
        )
        .unwrap();
    assert_eq!(caps.layout.leds.len(), 192);
    assert!(h.writes.is_empty()); // initialize sends nothing for fixed devices
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 192];
    m.frame(&colors, &mut h).unwrap();
    // each channel: 2 chunks (21 + 3) then SendUpdate; 8 channels x 24 LEDs
    assert_eq!(h.writes.len(), 24);
    let p0 = &h.writes[0];
    assert_eq!(p0.len(), 65);
    assert_eq!(p0[1], 0); // packet 0 + channel 0 * 6
    assert_eq!(&p0[2..8], &[2, 1, 3, 2, 1, 3]); // GRB: Nollie 8CH (0x1F01) swaps R/G
    let p2 = &h.writes[2];
    assert_eq!(p2[1], 0xff); // SendUpdate ends channel 0
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 24);
}

#[test]
fn fixed_packet_colors_and_intervals_follow_pid() {
    // upstream: NollieController.cpp SendPacketFS
    // Nollie 8CH (0x1F01) uses GRB, interval 6.
    let p = crate::report::fixed(0x1f01, 3, 1, &[Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&p[..6], &[0, 19, 2, 1, 3, 0]); // GRB
                                               // Nollie 28 12 (0x1616) uses RGB, interval 2.
    let p = crate::report::fixed(0x1616, 1, 0, &[Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&p[..6], &[0, 2, 1, 2, 3, 0]);
    // Nollie 1_OS2_1 (0x2A01) uses GRB, interval 30.
    let p = crate::report::fixed(0x2a01, 0, 0, &[Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&p[..6], &[0, 0, 2, 1, 3, 0]);
    // Prism8 8_OS2_1 (0x2C08) uses GRB, interval 6.
    let p = crate::report::fixed(0x2c08, 0, 4, &[Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&p[..6], &[0, 4, 2, 1, 3, 0]);
    assert_eq!(&crate::report::update()[..2], &[0, 0xff]);
}

#[test]
fn high_speed_builder_bytes_are_upstream_exact() {
    // upstream: NollieController.cpp SendPacket (packet builder; device path not ported)
    let p = crate::report::high_speed(15, &[Rgb { r: 1, g: 2, b: 3 }], 1);
    assert_eq!(&p[..8], &[0, 15, 1, 0, 1, 2, 1, 3]); // GRB, flag byte 2 = 1
    assert_eq!(p.len(), 1025);
}

#[test]
fn layouts_are_valid() {
    for row in 0..crate::devices_table::DETECTORS.len() {
        let mut m = crate::Nollie::default();
        let mut h = Fake { writes: Vec::new() };
        let id = crate::table::identify(&iface_for(row));
        match m.initialize(&iface_for(row), &mut h) {
            Ok(caps) => {
                assert!(id.supported);
                let sum: usize = caps.zones.iter().map(|z| z.leds).sum();
                assert_eq!(caps.layout.leds.len(), sum);
            }
            Err(_) => assert!(!id.supported, "row {} must match identify", row),
        }
    }
}
