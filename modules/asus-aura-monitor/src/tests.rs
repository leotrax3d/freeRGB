// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    let dets = crate::devices_table::DETECTORS;
    assert_eq!(dets.len(), 3);
    assert!(dets
        .iter()
        .all(|d| d.vid == 0x0b05 && d.usage_page == 0xffa0));
    let pids: Vec<u16> = dets.iter().map(|d| d.pid).collect();
    assert!(pids.contains(&0x1919) && pids.contains(&0x1968) && pids.contains(&0x19b9));
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.pid == pid)
        .unwrap();
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface,
        usage_page: d.usage_page,
        usage: d.usage,
        product: None,
    }
}

#[derive(Default)]
struct Fake {
    features: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, b: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        Ok(b.to_vec())
    }
}

#[test]
fn offset_and_begin_behavior_match_upstream() {
    // BeginUpdate emits two writes only for PG32UQ/XG32VC
    let v = crate::report::begin(0x19b9);
    assert_eq!(v.len(), 2);
    assert_eq!(&v[0][..5], &[0x03, 0x02, 0xa1, 0x80, 0x20]);
    assert_eq!(&v[1][..5], &[0x03, 0x02, 0xa1, 0x80, 0x30]);
    // Strix XG279Q uses offset 0 and no begin
    assert!(crate::report::begin(0x1919).is_empty());
    // UpdateLed uses offset 16 for Strix XG32VC
    let led0 = crate::report::update_led(0x1968, 0, Rgb { r: 1, g: 2, b: 3 });
    assert_eq!(&led0[0][..6], &[0x03, 0x02, 0xa1, 0x80, 16, 1]);
    assert_eq!(&led0[1][..6], &[0x03, 0x02, 0xa1, 0x80, 17, 3]);
    assert_eq!(&led0[2][..6], &[0x03, 0x02, 0xa1, 0x80, 18, 2]);
    let led0 = crate::report::update_led(0x1919, 0, Rgb { r: 1, g: 2, b: 3 });
    assert_eq!(&led0[0][4..6], &[0, 1]);
    assert_eq!(
        crate::report::apply(),
        &[0x03, 0x02, 0xa1, 0x80, 0xa0, 0x01, 0, 0]
    );
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x19b9), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 3);
    assert_eq!(caps.name, "ASUS ROG PG32UQ");
    // DeviceUpdateLEDs calls BeginUpdate(2), per-led(3*3), ApplyChanges
    let colors = vec![
        Rgb { r: 1, g: 2, b: 3 },
        Rgb { r: 4, g: 5, b: 6 },
        Rgb { r: 7, g: 8, b: 9 },
    ];
    m.frame(&colors, &mut h).unwrap();
    // 2 + 9 + 1 = 12 feature reports
    assert_eq!(h.features.len(), 12);
    assert_eq!(&h.features[0][..5], &[0x03, 0x02, 0xa1, 0x80, 0x20]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 12);
    let mut m = crate::M::default();
    m.initialize(&info_for(0x1919), &mut h).unwrap();
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 3], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 2], &mut h).is_err());
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x1919), &mut h).unwrap();
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert_eq!(led.name, format!("Backlight LED {}", n + 1));
    }
    assert_eq!(caps.zones.len(), 1);
    assert_eq!(caps.zones[0].leds, 3);
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let mut i = info_for(0x1919);
    i.usage_page = 0xff00;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    let mut i = info_for(0x1919);
    i.pid = 0xffff;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
}
