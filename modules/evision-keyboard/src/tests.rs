// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: 13 V1 rows + 2 V2 rows (Endorfy/V2), all interface 1, page 0xFF1C
    assert_eq!(crate::devices_table::DETECTORS.len(), 15);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.interface == 1 && d.usage_page == 0xff1c));
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| !d.v2)
            .count(),
        13
    );
    // upstream: matrix 6x23 (138 cells)
    assert_eq!(crate::devices_table::V1_MATRIX.len(), 138);
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.pid == pid)
        .unwrap();
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface as i32,
        usage_page: d.usage_page,
        usage: 0,
        product: None,
    }
}

#[derive(Default)]
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
fn v2_rows_are_not_supported() {
    // V2 keyboards need the keyvalue_map protocol (BEGIN/END CONFIGURE + read-back).
    for pid in [0x5041u16, 0x0012] {
        let id = crate::table::identify(&info_for(pid));
        assert!(!id.supported, "pid {pid}");
        assert!(id.reason.as_deref().unwrap().contains("not ported"));
    }
}

#[test]
fn v1_initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::EVision::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x5004), &mut h).unwrap();
    // upstream: 126 LEDs in a 6x23 matrix
    assert_eq!(caps.layout.leds.len(), 126);
    assert_eq!(caps.layout.width, 23);
    assert_eq!(caps.layout.height, 6);
    // LED 0 sits at the first non-NA matrix cell
    assert_eq!(caps.layout.leds[0].pos, (0, 0));
    assert!(h.writes.is_empty());
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 126];
    m.frame(&colors, &mut h).unwrap();
    // upstream: 378 color bytes in 7 x 0x36 chunks, checksummed
    assert_eq!(h.writes.len(), 7);
    let p0 = &h.writes[0];
    assert_eq!(p0[0], 4);
    assert_eq!(p0[3], 0x11);
    assert_eq!(p0[4], 0x36);
    assert_eq!(&p0[8..11], &[1, 2, 3]);
    let checksum: u16 = p0[3..].iter().map(|x| u16::from(*x)).sum();
    assert_eq!(p0[1], checksum as u8);
    assert_eq!(p0[2], (checksum >> 8) as u8);
    let last = &h.writes[6];
    assert_eq!(last[4], 0x36); // 378 = 7 x 54: the final chunk is full
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 7); // no exit packets in the color flow
}

#[test]
fn wrong_interface_or_usage_page_is_not_supported() {
    let mut i = info_for(0x5004);
    i.interface = 0;
    assert!(!crate::table::identify(&i).supported);
    let mut i2 = info_for(0x5004);
    i2.usage_page = 0x0001;
    assert!(!crate::table::identify(&i2).supported);
    let i3 = InterfaceInfo {
        vid: 0x1234,
        pid: 0x5004,
        interface: 1,
        usage_page: 0xff1c,
        usage: 0,
        product: None,
    };
    assert!(!crate::table::identify(&i3).supported);
}

#[test]
fn unsupported_initialize_fails_without_io() {
    let mut m = crate::EVision::default();
    let mut h = Fake::default();
    // unknown VID/PID combination
    let i = InterfaceInfo {
        vid: 0x1234,
        pid: 0x5004,
        interface: 1,
        usage_page: 0xff1c,
        usage: 0,
        product: None,
    };
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    // V2: unsupported and no writes
    assert!(m.initialize(&info_for(0x0012), &mut h).is_err());
    assert!(h.writes.is_empty());
    assert!(m.frame(&[], &mut h).is_err());
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::EVision::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x5004), &mut h).unwrap();
    for led in caps.layout.leds.iter() {
        assert!(led.pos.0 < 23 && led.pos.1 < 6);
    }
    // all 126 LEDs at distinct matrix cells
    let mut seen = std::collections::HashSet::new();
    for led in caps.layout.leds.iter() {
        assert!(seen.insert(led.pos));
    }
}
