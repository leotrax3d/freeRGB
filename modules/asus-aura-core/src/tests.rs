// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
#[derive(Default)]
struct Fake {
    writes: Vec<Vec<u8>>,
    config: Vec<u8>,
}
impl Hid for Fake {
    fn write(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(p.to_vec());
        Ok(p.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        Ok(self.config.clone())
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}
fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x0b05,
        pid,
        interface: -1,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}
#[test]
fn inventory_excludes_builtin() {
    assert!(devices_table::DETECTORS.iter().all(|d| d.pid != 0x19af));
    assert_eq!(devices_table::DETECTORS.len(), 5);
}
#[test]
fn direct_packet_is_byte_exact() {
    let p = report::direct(0, 0, &[Rgb { r: 1, g: 2, b: 3 }], true);
    assert_eq!(&p[..9], &[0xec, 0x40, 0x80, 0, 1, 1, 2, 3, 0]);
}
#[test]
fn init_and_chunked_frame() {
    let mut m = AsusAuraUsb::default();
    let mut h = Fake {
        config: {
            let mut x = vec![0; 65];
            x[0x1b] = 25;
            x
        },
        ..Default::default()
    };
    let c = m.initialize(&iface(0x1854), &mut h).unwrap();
    assert_eq!(c.layout.leds.len(), 25);
    m.frame(&[Rgb { r: 1, g: 2, b: 3 }; 25], &mut h).unwrap();
    assert_eq!(h.writes.len(), 3);
    assert_eq!(h.writes[0][1], 0xb0);
}
