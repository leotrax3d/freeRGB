// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
#[derive(Default)]
struct Fake {
    sent: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.sent.push(p.to_vec());
        Ok(p.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        panic!("Luxafor uses write only")
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        panic!("Luxafor uses write only")
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        panic!("Luxafor uses write only")
    }
}
fn iface(vid: u16, pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid,
        pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}
#[test]
fn direct_builder_is_byte_exact() {
    assert_eq!(report::direct(3, 1, 2, 3), [0, 1, 3, 1, 2, 3, 100, 0, 0]);
}
#[test]
fn matching_rejects_wrong_vid_or_pid() {
    assert!(table::identify(&iface(table::VID, table::PID)).supported);
    assert!(!table::identify(&iface(0xFFFF, table::PID)).supported);
    assert!(!table::identify(&iface(table::VID, 0xFFFF)).supported);
}
#[test]
fn initialize_frame_shutdown_sequence_is_simulated() {
    let mut m = Luxafor::default();
    let mut h = Fake::default();
    let c = m
        .initialize(&iface(table::VID, table::PID), &mut h)
        .unwrap();
    assert_eq!(c.layout.leds.len(), 6);
    m.frame(
        &(0..6).map(|n| Rgb { r: n, g: 2, b: 3 }).collect::<Vec<_>>(),
        &mut h,
    )
    .unwrap();
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.sent.len(), 6);
    assert_eq!(h.sent[0], report::direct(1, 0, 2, 3));
    assert_eq!(h.sent[5], report::direct(6, 5, 2, 3));
}
#[test]
fn layout_is_valid_and_frame_size_is_checked() {
    let mut m = Luxafor::default();
    let mut h = Fake::default();
    m.initialize(&iface(table::VID, table::PID), &mut h)
        .unwrap();
    assert!(m.frame(&[], &mut h).is_err());
}
