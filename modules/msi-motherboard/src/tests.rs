// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
use freergb_module_api::sdk::Hid;

#[derive(Default)]
struct Fake {
    get_len: usize,
    sent: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.sent.push(p.to_vec());
        Ok(p.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        Ok(vec![0; self.get_len])
    }
}
fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1462,
        pid,
        interface: 1,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}
fn common_iface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x0db0,
        pid: 0x0076,
        interface: 0xff00,
        usage_page: 1,
        usage: 0,
        product: None,
    }
}

#[test]
fn generated_inventory_has_all_upstream_detectors() {
    assert_eq!(devices_table::DETECTORS.len(), 124);
}

#[test]
fn zone_packet_is_byte_exact() {
    let p = report::zone_packet(
        report::REPORT_112,
        &[Rgb { r: 1, g: 2, b: 3 }],
        &[1, 0, 0, 0],
    );
    assert_eq!(&p[..12], &[0x52, 1, 1, 2, 3, 40, 1, 2, 3, 0x80, 0, 1]);
    assert_eq!(p.len(), 112);
}

#[test]
fn initializes_185_and_sends_feature_frame() {
    let mut m = MsiMotherboard::default();
    let mut h = Fake {
        get_len: 185,
        ..Default::default()
    };
    let c = m.initialize(&iface(0x1720), &mut h).unwrap();
    assert_eq!(c.zones.len(), 7);
    m.frame(&vec![Rgb { r: 9, g: 8, b: 7 }; c.layout.leds.len()], &mut h)
        .unwrap();
    assert_eq!(h.sent.last().unwrap().len(), 185);
    assert_eq!(h.sent.last().unwrap()[0], 0x52);
}

#[test]
fn optional_zones_change_layout() {
    let mut m = MsiMotherboard::default();
    let mut h = Fake {
        get_len: 112,
        ..Default::default()
    };
    let c = m
        .initialize_with_zones(
            &iface(0x1720),
            Some(&[Zone {
                name: "JRGB1".into(),
                leds: 4,
            }]),
            &mut h,
        )
        .unwrap();
    assert_eq!(c.layout.leds.len(), 4);
    assert_eq!(c.zones[0].leds, 4);
}

#[test]
fn initializes_761_with_configuration_and_packets() {
    let mut m = MsiMotherboard::default();
    let mut h = Fake {
        get_len: 0,
        ..Default::default()
    };
    let c = m.initialize(&common_iface(), &mut h).unwrap();
    assert_eq!(c.layout.leds.len(), 120);
    assert_eq!(h.sent[0][0], 0x50);
    m.frame(&vec![Rgb { r: 1, g: 2, b: 3 }; c.layout.leds.len()], &mut h)
        .unwrap();
    assert_eq!(h.sent.len(), 5);
    assert!(h
        .sent
        .iter()
        .skip(1)
        .all(|p| p[0] == 0x51 && p.len() == 761));
}
