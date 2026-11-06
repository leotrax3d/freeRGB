// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{ErrorCode, InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: MSIMonitorControllerDetect.cpp registers one
    // REGISTER_HID_DETECTOR_IPU row (interface 0, usage page 0x01, usage 0).
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1462 && d.pid == 0x3fa4 && d.interface == 0));
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

/// 64-byte version query reply with ACK 0x5A and version at byte 3.
fn version_reply(v: u8) -> Vec<u8> {
    let mut r = vec![0u8; 64];
    r[1] = 0x5a;
    r[3] = v;
    r
}

struct Fake {
    writes: Vec<Vec<u8>>,
    features: Vec<Vec<u8>>,
    reads: Option<Vec<Vec<u8>>>,
}
impl Hid for Fake {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        let mut q = self.reads.take().unwrap_or_default();
        if q.is_empty() {
            return Err(ModuleError::new(ErrorCode::Disconnected, "no queued reads"));
        }
        Ok(q.remove(0))
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn packets_are_byte_exact() {
    // upstream: MSIMonitorController.cpp GetLayoutVersion request
    let q = crate::report::version_query();
    assert_eq!(q[0], 0x01);
    assert_eq!(q[1], 0xb0);
    assert!(&q[2..].iter().all(|b| *b == 0));
    // version parse requires the 0x5A ACK at byte 1
    assert_eq!(
        crate::report::version_from_reply(&version_reply(34)),
        Some(34)
    );
    let mut no_ack = vec![0u8; 64];
    no_ack[3] = 34;
    assert_eq!(crate::report::version_from_reply(&no_ack), None);
    // 168-byte 0x72 frame for version 34 (9 LEDs, left-to-right)
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 9];
    colors[0] = Rgb { r: 1, g: 2, b: 3 };
    colors[8] = Rgb { r: 9, g: 8, b: 7 };
    let f = crate::report::frame(&colors, false);
    assert_eq!(f.len(), 168);
    assert_eq!(f[0], 0x72);
    // control blocks at index 1 and 12
    assert_eq!(&f[1..12], &[1, 1, 2, 3, 1, 100, 0, 0, 0, 0x80, 0x00]);
    assert_eq!(&f[12..23], &[1, 1, 2, 3, 1, 100, 0, 0, 0, 0x80, 0x00]);
    // second inline colour array at index 95
    assert_eq!(&f[95..98], &[1, 2, 3]);
    assert_eq!(&f[95 + 24..95 + 27], &[9, 8, 7]);
    // first array at index 23 stays clear on single-block layouts
    assert!(&f[23..95].iter().all(|b| *b == 0));
    // no flash write while streaming
    assert_eq!(f[167], 0x00);
}

#[test]
fn layouts_match_the_upstream_table() {
    // upstream verified row: version 34, 9 LEDs, left-to-right
    assert_eq!(crate::report::layout(34), Some((9, false)));
    // right-to-left rows mirror
    assert_eq!(crate::report::layout(5), Some((12, true)));
    assert_eq!(crate::report::layout(29), Some((24, true)));
    // circle plain line
    assert_eq!(crate::report::layout(8), Some((24, false)));
    // dual block versions stay disabled upstream
    assert_eq!(crate::report::layout(11), None);
    assert_eq!(crate::report::layout(13), None);
    assert_eq!(crate::report::layout(15), None);
    // advanced 0x82 versions are not addable
    assert_eq!(crate::report::layout(31), None);
    assert_eq!(crate::report::layout(47), None);
    // legacy 0x71 versions fall through unsupported
    assert_eq!(crate::report::layout(1), None);
    assert_eq!(crate::report::layout(6), None);
    assert_eq!(crate::report::layout(27), None);
    assert_eq!(crate::report::LAYOUTS.len(), 27);
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        features: Vec::new(),
        reads: Some(vec![version_reply(34)]),
    };
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 9);
    assert_eq!(caps.firmware.as_deref(), Some("34"));
    assert!(!m.reverse);
    // version query write
    assert_eq!(h.writes.len(), 1);
    assert_eq!(h.writes[0][..2], [0x01, 0xb0]);
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 9];
    colors[1] = Rgb { r: 4, g: 5, b: 6 };
    m.frame(&colors, &mut h).unwrap();
    // one 168-byte feature report per frame
    assert_eq!(h.features.len(), 1);
    assert_eq!(h.features[0].len(), 168);
    assert_eq!(&h.features[0][98..101], &[4, 5, 6]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 1); // upstream sends no exit packets
    assert!(m.frame(&colors, &mut h).is_err());
}

#[test]
fn right_to_left_layout_mirrors_the_frame() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        features: Vec::new(),
        reads: Some(vec![version_reply(5)]), /* MAG272R, 12 LEDs RTL */
    };
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 12);
    assert!(m.reverse);
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 12];
    colors[0] = Rgb { r: 1, g: 0, b: 0 };
    colors[11] = Rgb { r: 0, g: 0, b: 2 };
    m.frame(&colors, &mut h).unwrap();
    // mirrored: OpenRGB LED 0 lands in the last array slot
    assert_eq!(&h.features[0][95 + 33..95 + 36], &[1, 0, 0]);
    assert_eq!(&h.features[0][95..98], &[0, 0, 2]);
}

#[test]
fn unsupported_versions_fail_closed() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        features: Vec::new(),
        reads: None, // no reply => legacy 0x71 controller
    };
    assert!(m.initialize(&info_for(), &mut h).is_err());
    // ACK missing
    let mut bad = vec![0u8; 64];
    bad[3] = 34;
    m = crate::M::default();
    h.reads = Some(vec![bad]);
    assert!(m.initialize(&info_for(), &mut h).is_err());
    // legacy and dual-block versions
    for v in [1u8, 6, 11, 13, 15, 31, 47, 100] {
        m = crate::M::default();
        h.reads = Some(vec![version_reply(v)]);
        assert!(m.initialize(&info_for(), &mut h).is_err(), "version {v}");
    }
}

#[test]
fn wrong_interface_or_usage_is_not_supported() {
    let mut i = info_for();
    i.interface = 1;
    assert!(!crate::table::identify(&i).supported);
    i = info_for();
    i.usage_page = 0xff00;
    assert!(!crate::table::identify(&i).supported);
    i = info_for();
    i.vid = 0x1234;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        features: Vec::new(),
        reads: Some(vec![version_reply(22)]), /* 24 LEDs */
    };
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert_eq!(led.pos.0, n as u16);
        assert_eq!(led.name, format!("LED {}", n + 1));
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 24], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 23], &mut h).is_err());
    // more colors than the 24 array slots are impossible by construction
    assert_eq!(caps.layout.leds.len(), crate::report::ARRAY_SLOTS);
}
