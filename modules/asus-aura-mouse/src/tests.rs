// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

fn info_for(pid: u16) -> InterfaceInfo {
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.pid == pid);
    match d {
        Some(d) => InterfaceInfo {
            vid: d.vid,
            pid: d.pid,
            interface: d.interface,
            usage_page: d.usage_page,
            usage: 0,
            product: d.product.map(String::from),
        },
        None => InterfaceInfo {
            vid: 0x0b05,
            pid,
            interface: 0,
            usage_page: 0xff01,
            usage: 0,
            product: None,
        },
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
fn inventory_is_complete() {
    // upstream: AsusAuraMouseDevices.h direct=true devices with a usable
    // aura_mouse_led_maps entry; the Spatha X Dock product on 0x1979 stays
    // excluded because its direct=false.
    assert_eq!(crate::devices_table::DETECTORS.len(), 9);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x0b05 && d.usage_page == 0xff01 && d.interface == 0));
}

#[test]
fn packets_are_byte_exact() {
    // upstream: AuraMouseController.cpp SendDirect
    let packets = crate::report::direct(&[
        Rgb { r: 1, g: 2, b: 3 },
        Rgb { r: 4, g: 5, b: 6 },
        Rgb { r: 7, g: 8, b: 9 },
        Rgb {
            r: 10,
            g: 11,
            b: 12,
        },
        Rgb {
            r: 13,
            g: 14,
            b: 15,
        },
        Rgb {
            r: 16,
            g: 17,
            b: 18,
        },
    ]);
    assert_eq!(packets.len(), 2);
    assert_eq!(&packets[0][..9], &[0, 0x51, 0x29, 5, 0, 0, 1, 2, 3]);
    assert_eq!(&packets[0][18..21], &[13, 14, 15]);
    assert_eq!(&packets[1][..9], &[0, 0x51, 0x29, 1, 0, 5, 16, 17, 18]);
}

#[test]
fn spatha_zone_slots_are_byte_exact() {
    // upstream: aura_mouse_led_maps Spatha X entry paints underglow into
    // slots 2/3/4 and sends the 5-colour buffer in one packet.
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x1977), &mut h).unwrap();
    assert_eq!(
        caps.layout
            .leds
            .iter()
            .map(|l| l.name.clone())
            .collect::<Vec<_>>(),
        vec!["Logo LED", "Scroll Wheel LED", "Underglow LED"]
    );
    let colors = vec![
        Rgb { r: 1, g: 1, b: 1 },
        Rgb { r: 2, g: 2, b: 2 },
        Rgb { r: 3, g: 3, b: 3 },
    ];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 1);
    let p = &h.writes[0];
    assert_eq!(&p[..9], &[0, 0x51, 0x29, 5, 0, 0, 1, 1, 1]);
    assert_eq!(&p[9..12], &[2, 2, 2]);
    assert_eq!(&p[12..21], &[3, 3, 3, 3, 3, 3, 3, 3, 3]);
}

#[test]
fn chakram_x_one_led_spreads_twelve_slots() {
    // upstream: Chakram X: 9 colour slots in two packets (5 + 4), underglow
    // paints slots 0..6, logo 7, scroll 8.
    let mut m = crate::M::default();
    let mut h = Fake::default();
    m.initialize(&info_for(0x1a18), &mut h).unwrap();
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 3];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2);
    assert_eq!(&h.writes[0][..9], &[0, 0x51, 0x29, 5, 0, 0, 1, 2, 3]);
    // packet 0 carries slots 0..4 (all underglow (1,2,3))
    assert_eq!(
        &h.writes[0][6..21],
        &[1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3]
    );
    // packet 1 carries slots 5..8: underglow 5/6, logo, scroll
    assert_eq!(&h.writes[1][..6], &[0, 0x51, 0x29, 4, 0, 5]);
    assert_eq!(&h.writes[1][6..18], &[1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3]);
    // everything past the colours is zero
    assert!(&h.writes[1][18..].iter().all(|b| *b == 0));
}

#[test]
fn single_led_devices_send_one_packet() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    m.initialize(&info_for(0x1a70), &mut h).unwrap();
    m.frame(&[Rgb { r: 9, g: 8, b: 7 }], &mut h).unwrap();
    assert_eq!(h.writes.len(), 1);
    assert_eq!(&h.writes[0][..9], &[0, 0x51, 0x29, 1, 0, 0, 9, 8, 7]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 1); // upstream sends no exit packets
}

#[test]
fn unsupported_devices_are_rejected() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    for pid in [0x1845u16, 0x1847, 0x1910] {
        let mut h2 = Fake::default();
        assert!(
            m.initialize(&info_for(pid), &mut h2).is_err(),
            "pid {pid:#x}"
        );
        assert!(!crate::table::identify(&info_for(pid)).supported);
    }
    // the Spatha X Dock product shares PID 0x1979 and stays unsupported
    let mut i = info_for(0x1979);
    i.product = Some("Asus ROG Spatha X Dock".into());
    assert!(m.initialize(&i, &mut h).is_err());
    assert!(!crate::table::identify(&i).supported);
    // wrong usage page / interface
    let mut i = info_for(0x1977);
    i.usage_page = 0xff02;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x1a88), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert!(!led.name.is_empty());
        let _ = n;
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 2], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 1], &mut h).is_err());
}
