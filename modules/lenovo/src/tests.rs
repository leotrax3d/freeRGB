// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: 24 detector rows (8 four-zone, k500, k510, m300, 5 legacy, 8 gen7)
    assert_eq!(crate::devices_table::DETECTORS.len(), 24);
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.pid == pid)
        .unwrap_or(&crate::devices_table::DETECTORS[0]);
    InterfaceInfo {
        vid: d.vid,
        pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

#[derive(Default)]
struct Fake {
    features: Vec<Vec<u8>>,
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
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn supported_set_matches_the_audit() {
    // four-zone, k500, m300, legacy ANSI (C935/C957/C967) and Gen7 C977 supported;
    // k510, Gen10 (C195/C197), Gen7 laptops and Y750 (C956) unsupported.
    for d in crate::devices_table::DETECTORS.iter() {
        let id = crate::table::identify(&info_for(d.pid));
        assert_eq!(
            id.supported,
            crate::table::SUPPORTED.contains(&(d.pid, d.protocol)),
            "row {} ({})",
            d.pid,
            d.protocol
        );
    }
    // 14 supported detector rows
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| crate::table::SUPPORTED.contains(&(d.pid, d.protocol)))
            .count(),
        14
    );
    // k510 is not supported because frame sends no packets
    let k510 = crate::table::identify(&info_for(0x619a));
    assert!(!k510.supported);
    assert!(k510.reason.as_deref().unwrap().contains("not fully ported"));
    // Gen10 header/zone-ids missing
    assert!(!crate::table::identify(&info_for(0xc195)).supported);
    assert!(!crate::table::identify(&info_for(0xc197)).supported);
    assert!(!crate::table::identify(&info_for(0xc956)).supported);
    assert!(!crate::table::identify(&info_for(0xc978)).supported);
}

#[test]
fn legacy_packets_are_byte_exact() {
    // upstream: LenovoUSBController.cpp sendLedsDirect start 0xA0+zone, 47 LEDs/packet
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 108];
    let p = crate::report::legacy(0xC935, &colors);
    assert_eq!(p.len(), 3); // Y740: 108 LEDs in 47+47+14 chunks
    assert_eq!(&p[0][..5], &[0x07, 0xA1, 47, 0, 110]); // first LED id 0x6E
    assert_eq!(&p[0][5..8], &[1, 2, 3]);
}

#[test]
fn gen7_packets_are_byte_exact() {
    // upstream: LenovoUSBController_Gen7_8.cpp setLedsDirect (non-Gen10)
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 101];
    let p = crate::report::gen7(0xC977, &colors);
    assert_eq!(&p[..10], &[0x07, 0xA1, 0xC0, 0x03, 1, 0, 1, 2, 3, 2]); // entry: id, zone, r, g, b, next id
}

#[test]
fn four_zone_m300_k500_packets_are_byte_exact() {
    // upstream: Lenovo4ZoneUSBController.cpp static mode frame
    assert_eq!(
        &crate::report::four_zone(&[Rgb { r: 1, g: 2, b: 3 }; 4])[..8],
        &[0xCC, 0x16, 1, 1, 1, 1, 2, 3]
    );
    // upstream: LenovoM300Controller.cpp static frame with checksum
    let m = crate::report::m300(Rgb { r: 1, g: 2, b: 3 });
    assert_eq!(&m[..8], &[0, 0x25, 0, 1, 1, 0x0c, 3, 0x64]);
    assert_eq!(&m[9..12], &[1, 2, 3]);
    // upstream: LenovoK500Controller.cpp 9-byte setup feature
    let k = crate::report::k500(&[Rgb { r: 1, g: 2, b: 3 }]);
    assert_eq!(&k[0][..5], &[0, 0x14, 0, 0x5a, 0]);
}

#[test]
fn unsupported_rows_fail_initialize_without_io() {
    for pid in [0x619Au16, 0xC195, 0xC197, 0xC956, 0xC978] {
        let mut m = crate::Lenovo::default();
        let mut h = Fake::default();
        assert!(m.initialize(&info_for(pid), &mut h).is_err());
        assert!(h.features.is_empty() && h.writes.is_empty());
    }
}

#[test]
fn k500_matrix_and_layouts_are_valid() {
    // upstream: LenovoK500Controller.cpp k500_matrix has 104 (byte,bit) pairs
    assert_eq!(crate::k500_matrix::MATRIX.len(), 104);
    // legacy id tables match the upstream device data (checked against LenovoDevices.h)
    assert_eq!(crate::legacy_ids::Y740_ALL.len(), 108);
    assert_eq!(crate::legacy_ids::Y750_ALL.len(), 148);
    assert_eq!(crate::legacy_ids::Y760_KEYBOARD.len(), 145);
    for supported_pid in [0xC963u16, 0x60D5, 0x60E4, 0xC977] {
        let mut m = crate::Lenovo::default();
        let mut h = Fake::default();
        let caps = m.initialize(&info_for(supported_pid), &mut h).unwrap();
        assert!(!caps.layout.leds.is_empty());
    }
}
