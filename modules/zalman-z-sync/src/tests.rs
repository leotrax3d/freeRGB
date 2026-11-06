// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{ErrorCode, InterfaceInfo, ModuleError, Zone};

#[test]
fn inventory_is_complete() {
    // upstream: ZalmanZSyncControllerDetect.cpp registers one plain
    // REGISTER_HID_DETECTOR for 0x1C57:0x7ED0.
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1c57 && d.pid == 0x7ed0));
}

fn info_for() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
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
    read: Option<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        if self.read.as_ref().is_none() {
            return Err(ModuleError::new(ErrorCode::Disconnected, "no queued reads"));
        }
        self.read
            .take()
            .ok_or_else(|| ModuleError::new(ErrorCode::Disconnected, "no queued reads"))
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
    // upstream: ZalmanZSyncController.cpp SendPortState / SendDirect /
    // SendCommit / SendFirmwareRequest / SendReset
    assert_eq!(&crate::report::firmware_request()[0..2], &[0, 0x02]);
    assert_eq!(&crate::report::state(1)[0..4], &[0, 0x38, 1, 2]);
    assert_eq!(
        &crate::report::direct(3, 12, &[Rgb { r: 1, g: 2, b: 3 }], 2)[0..7],
        &[0, 0x32, 3, 12, 1, 2, 3]
    );
    assert_eq!(&crate::report::commit()[0..3], &[0, 0x33, 0xff]);
    assert_eq!(&crate::report::reset(4)[0..3], &[0, 0x37, 4]);
}

#[test]
fn firmware_string_matches_upstream() {
    // upstream: SendFirmwareRequest maps a byte 0x03 < 112 onto "0.7.1"
    assert_eq!(
        crate::report::firmware_string(&[0, 0, 9, 96]),
        Some("0.7.1".into())
    );
    assert_eq!(
        crate::report::firmware_string(&[0, 0, 1, 0x7A]),
        Some("1.7.10".into())
    );
    assert_eq!(crate::report::firmware_string(&[0, 0]), None);
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        read: Some(vec![0, 0, 1, 0x7a]),
    };
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    // default: one LED per channel
    assert_eq!(caps.zones.len(), 8);
    assert_eq!(caps.layout.leds.len(), 8);
    assert_eq!(caps.zones[3].name, "Addressable RGB Header 4");
    // firmware request write at init
    assert_eq!(h.writes[0][..2], [0, 0x02]);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 8];
    m.frame(&colors, &mut h).unwrap();
    // per channel: state + 3 planes + commit = 5 writes, 8 channels
    assert_eq!(h.writes.len(), 1 + 8 * 5);
    h.writes.remove(0);
    let f = &h.writes;
    for ch in 0..8u8 {
        let b = ch as usize * 5;
        assert_eq!(&f[b][..4], &[0, 0x38, ch, 2]);
        assert_eq!(&f[b + 1][..7], &[0, 0x32, ch, 0, 1, 0, 1]);
        assert_eq!(&f[b + 2][..7], &[0, 0x32, ch, 0, 1, 1, 2]);
        assert_eq!(&f[b + 3][..7], &[0, 0x32, ch, 0, 1, 2, 3]);
        assert_eq!(&f[b + 4][..3], &[0, 0x33, 0xff]);
    }
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 8 * 5); // upstream sends no exit packets
}

#[test]
fn unsupported_device_fails_initialize_without_io() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        read: None,
    };
    let mut i = info_for();
    i.vid = 0x1234;
    assert!(m.initialize(&i, &mut h).is_err());
    assert!(h.writes.is_empty());
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn zones_are_layouted_and_clamped() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        read: None,
    };
    // wrong zone count rejected
    assert!(m
        .initialize_with_zones(
            &info_for(),
            Some(&[Zone {
                name: "A".into(),
                leds: 5
            }]),
            &mut h
        )
        .is_err());
    // sizes clamp to upstream leds_max 40
    let mut zones: Vec<Zone> = (0..8)
        .map(|n| Zone {
            name: format!("Z{n}"),
            leds: 50,
        })
        .collect();
    zones[7].leds = 40;
    let caps = m
        .initialize_with_zones(&info_for(), Some(&zones), &mut h)
        .unwrap();
    assert_eq!(caps.zones[0].leds, 40);
    assert_eq!(caps.layout.leds.len(), 320);
    // frame across multiple chunks with 50-LED blocks (clamped 40)
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; 320];
    m.frame(&colors, &mut h).unwrap();
    // channel 0 (40 LEDs): writes[0] is the firmware request
    let base0 = 1usize;
    assert_eq!(h.writes[base0][..4], [0, 0x38, 0, 2]);
    assert_eq!(&h.writes[base0 + 1][..7], &[0, 0x32, 0, 0, 40, 0, 0]);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        read: None,
    };
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert_eq!(led.pos.1 as usize, n); // one LED per channel row
        assert_eq!(led.name, format!("Addressable RGB Header {}, LED 1", n + 1));
    }
}
