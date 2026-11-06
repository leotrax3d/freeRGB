// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct Fake {
    writes: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, data: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(data.to_vec());
        Ok(data.len())
    }
    fn read(&mut self, len: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        Ok(vec![0u8; len])
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

fn info_ok() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x2516,
        pid: 0x01B5,
        interface: 1,
        usage_page: 0xFF00,
        usage: 0,
        product: None,
    }
}

#[test]
fn identify_matches_only_registered_interface() {
    assert!(crate::table::identify(&info_ok()).supported);
    let mut i = info_ok();
    i.interface = 0;
    assert!(!crate::table::identify(&i).supported);
    i = info_ok();
    i.usage_page = 0x00FF;
    assert!(!crate::table::identify(&i).supported);
    i = info_ok();
    i.pid = 0x01B6;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn init_sequence_is_byte_exact() {
    let mut m = crate::IntelArcA770Le::default();
    let mut h = Fake { writes: Vec::new() };
    let caps = m.initialize(&info_ok(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 91);
    assert_eq!(caps.zones.len(), 5);
    assert_eq!(h.writes.len(), 3);
    assert_eq!(h.writes[0], crate::report::firmware_query());
    assert_eq!(h.writes[1], crate::report::enable());
    assert_eq!(h.writes[2], crate::report::apply());
    m.shutdown(None, &mut h).unwrap();
}

#[test]
fn frame_chunks_and_marker_bytes_are_exact() {
    let mut m = crate::IntelArcA770Le::default();
    let mut h = Fake { writes: Vec::new() };
    m.initialize(&info_ok(), &mut h).unwrap();

    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 91];
    colors[0] = Rgb { r: 7, g: 8, b: 9 };
    colors[90] = Rgb { r: 7, g: 8, b: 9 };

    h.writes.clear();
    m.frame(&colors, &mut h).unwrap();

    assert_eq!(h.writes.len(), 7);
    for p in &h.writes[..6] {
        assert_eq!(p.len(), 65);
        assert_eq!(p[3], 15);
        assert_eq!(&p[0..5], &[0x00, 0xC0, 0x01, 15, 0x00]);
    }
    let last = &h.writes[6];
    assert_eq!(last.len(), 65);
    assert_eq!(last[3], 1);
    assert_eq!(&last[0..5], &[0x00, 0xC0, 0x01, 1, 0x00]);
    assert_eq!(last[5], 0x96);
    // logo: max channel in red, G/B forced zero
    assert_eq!(last[6], 9);
    assert_eq!(last[7], 0);
    assert_eq!(last[8], 0);

    // global color index 0 -> marker 0x01 in first packet, R/G/B at +1/+2/+3
    assert_eq!(h.writes[0][5], 0x01);
    assert_eq!(h.writes[0][6], 7);
    assert_eq!(h.writes[0][7], 8);
    assert_eq!(h.writes[0][8], 9);

    assert!(m
        .frame(&vec![Rgb { r: 0, g: 0, b: 0 }; 90], &mut h)
        .is_err());
}

#[test]
fn report_lists_and_counts() {
    let packets = crate::report::frames(&vec![Rgb { r: 0, g: 0, b: 0 }; 91]);
    assert_eq!(packets.len(), 7);
    assert!(packets.iter().all(|p| p.len() == 65));
    assert_eq!(packets[6][3], 1);
}
