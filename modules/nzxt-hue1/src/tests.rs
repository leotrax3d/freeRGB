// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: NZXTHue1ControllerDetect.cpp registers a single plain
    // REGISTER_HID_DETECTOR for the Smart Device V1 (0x1E71:0x1714).
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .find(|d| d.vid == 0x1e71 && d.pid == 0x1714),
        Some(&crate::devices_table::DETECTORS[0])
    );
}

fn info_for() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface,
        usage_page: d.usage_page,
        usage: d.usage,
        product: None,
    }
}

struct Fake {
    writes: Vec<Vec<u8>>,
    reads: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, len: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        if self.reads.is_empty() {
            return Err(ModuleError::new(
                freergb_module_api::ErrorCode::Disconnected,
                "no queued reads",
            ));
        }
        Ok(self.reads.remove(0).into_iter().take(len).collect())
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

/// Status packet with 2 Aer fans (accessory byte 0x10 = 0x08 => type 1),
/// firmware 3.6.
fn status_packet() -> Vec<u8> {
    let mut v = vec![0u8; 21];
    v[0x0b] = 3;
    v[0x0e] = 6;
    v[0x10] = 0x08;
    v[0x11] = 2;
    v
}

#[test]
fn packets_are_byte_exact() {
    // upstream: NZXTHue1Controller.cpp Initialize / SendPacket
    assert_eq!(&crate::report::init()[0..2], &[0x01, 0x5c]);
    assert_eq!(&crate::report::start_reporting()[0..2], &[0x01, 0x5d]);
    let p = &crate::report::direct(&[Rgb { r: 1, g: 2, b: 3 }])[0];
    assert_eq!(&p[0..5], &[0x02, 0x4b, 0x00, 0x00, 0x00]);
    // G at +0, R at +1, B at +2
    assert_eq!(&p[5..8], &[2, 1, 3]);
    let p = &crate::report::direct(&[Rgb { r: 1, g: 2, b: 3 }])[1];
    assert_eq!(p[0], 0x03);
    // fewer than 20 colors: packet 2 carries no colors
    assert!(&p[1..].iter().all(|b| *b == 0));
}

/// Expected wire byte for color `i`, channel `o` (0: G, 1: R, 2: B) when
/// colors[i] = { r: i, g: i+1, b: i+2 }; upstream sends G first.
fn wb(i: usize, o: usize) -> u8 {
    match o {
        0 => (i + 1) as u8,
        1 => i as u8,
        _ => (i + 2) as u8,
    }
}

#[test]
fn two_packet_split_is_byte_exact() {
    // upstream: NZXTHue1Controller.cpp SendPacket splits at 20 colors.
    let colors: Vec<Rgb> = (0..25)
        .map(|i| Rgb {
            r: i,
            g: i + 1,
            b: i + 2,
        })
        .collect();
    let packets = crate::report::direct(&colors);
    assert_eq!(packets.len(), 2);
    let p1 = &packets[0];
    for (k, b) in p1[5..5 + 60].iter().enumerate() {
        assert_eq!(*b, wb(k / 3, k % 3));
    }
    let p2 = &packets[1];
    // upstream copies the remaining colors from the start of color_data
    for (k, b) in p2[1..1 + 15].iter().enumerate() {
        assert_eq!(*b, wb(k / 3, k % 3));
    }
    assert!(&p2[16..].iter().all(|b| *b == 0));
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![status_packet()],
    };
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    // 2 fans x 8 LEDs
    assert_eq!(caps.layout.leds.len(), 16);
    assert_eq!(caps.firmware.as_deref(), Some("3.6"));
    assert_eq!(caps.name, "NZXT Smart Device V1");
    // init + start reporting
    assert_eq!(h.writes[0][..2], [0x01, 0x5c]);
    assert_eq!(h.writes[1][..2], [0x01, 0x5d]);
    let mut colors = vec![Rgb { r: 1, g: 2, b: 3 }; 16];
    colors[14].r = 4;
    colors[15].g = 5;
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 4);
    let f1 = &h.writes[2];
    assert_eq!(&f1[..5], &[0x02, 0x4b, 0x00, 0x00, 0x00]);
    assert_eq!(&f1[5..8], &[2, 1, 3]);
    // LED 15 (index 14, r=4) and LED 16 (index 15, g=5) land at bytes 47..53
    // of packet 1 in G/R/B order
    assert_eq!(&f1[47..53], &[2, 4, 3, 5, 1, 3]);
    let f2 = &h.writes[3];
    assert_eq!(f2[0], 0x03);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 4); // upstream sends no exit packets
    assert!(m.frame(&colors, &mut h).is_err());
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![status_packet()],
    };
    // unknown vendor
    let mut i = info_for();
    i.vid = 0x9999;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    // unknown PID
    let mut i = info_for();
    i.pid = 0x9999;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn missing_status_packet_fails_closed() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![],
    };
    assert!(m.initialize(&info_for(), &mut h).is_err());
    // zero device count: upstream would yield 0 LEDs, the module refuses
    h.writes.clear();
    h.reads = vec![status_packet()];
    h.reads[0][0x11] = 0;
    assert!(m.initialize(&info_for(), &mut h).is_err());
}

#[test]
fn oversize_layout_fails_closed() {
    // 8 strips x 10 LEDs exceeds the 40-color Direct-mode packet capacity.
    let mut m = crate::M::default();
    let mut pkt = status_packet();
    pkt[0x10] = 0x00; // strip
    pkt[0x11] = 8;
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![pkt],
    };
    assert!(m.initialize(&info_for(), &mut h).is_err());
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![status_packet()],
    };
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert_eq!(led.pos.0, n as u16);
        assert_eq!(led.name, format!("Hue 1 Channel, LED {}", n + 1));
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 16], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 15], &mut h).is_err());
}
