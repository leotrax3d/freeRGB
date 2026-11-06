// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
#[derive(Default)]
struct Fake {
    sent: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        panic!("ThingM uses feature reports")
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        panic!("ThingM uses feature reports")
    }
    fn send_feature(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.sent.push(p.to_vec());
        Ok(p.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        panic!("ThingM does not read")
    }
}
fn iface(page: u16, usage: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: devices_table::VID,
        pid: devices_table::PID,
        interface: 0,
        usage_page: page,
        usage,
        product: None,
    }
}
#[test]
fn direct_builder_is_byte_exact() {
    assert_eq!(report::direct(2, 1, 2, 3), [1, 0x63, 1, 2, 3, 0, 0, 2, 0]);
}
#[test]
fn matching_rejects_wrong_usage() {
    assert!(table::identify(&iface(devices_table::USAGE_PAGE, devices_table::USAGE)).supported);
    assert!(!table::identify(&iface(1, devices_table::USAGE)).supported);
    assert!(!table::identify(&iface(devices_table::USAGE_PAGE, 2)).supported);
}
#[test]
fn initialize_frame_shutdown_sequence_is_simulated() {
    let mut m = ThingmBlink::default();
    let mut h = Fake::default();
    let caps = m
        .initialize(
            &iface(devices_table::USAGE_PAGE, devices_table::USAGE),
            &mut h,
        )
        .unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    m.frame(
        &[Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }],
        &mut h,
    )
    .unwrap();
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(
        h.sent,
        vec![
            report::direct(1, 1, 2, 3).to_vec(),
            report::direct(2, 4, 5, 6).to_vec()
        ]
    );
}
#[test]
fn layout_and_frame_size_are_validated() {
    let mut m = ThingmBlink::default();
    let mut h = Fake::default();
    let c = m
        .initialize(
            &iface(devices_table::USAGE_PAGE, devices_table::USAGE),
            &mut h,
        )
        .unwrap();
    c.layout.validate().unwrap();
    assert!(m.frame(&[], &mut h).is_err());
}
