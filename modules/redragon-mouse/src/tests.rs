// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
#[derive(Default)]
struct Fake {
    sent: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        panic!("feature reports only")
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        panic!("feature reports only")
    }
    fn send_feature(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.sent.push(p.to_vec());
        Ok(p.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        panic!("no reads")
    }
}
fn iface(pid: u16) -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x04D9,
        pid,
        interface: 2,
        usage_page: 0xFFA0,
        usage: 0,
        product: None,
    }
}
#[test]
fn report_builders_are_byte_exact() {
    assert_eq!(
        report::profile_zero(),
        [2, 243, 44, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        report::color(1, 2, 3),
        [2, 243, 73, 4, 3, 0, 0, 0, 1, 2, 3, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        report::apply(),
        [2, 241, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}
#[test]
fn matching_rejects_wrong_interface_or_usage() {
    assert!(table::identify(&iface(0xFC30)).supported);
    let mut i = iface(0xFC30);
    i.interface = 1;
    assert!(!table::identify(&i).supported);
    i = iface(0xFC30);
    i.usage_page = 1;
    assert!(!table::identify(&i).supported);
}
#[test]
fn initialize_frame_shutdown_sequence_is_simulated() {
    let mut m = RedragonMouse::default();
    let mut h = Fake::default();
    let c = m.initialize(&iface(0xFC30), &mut h).unwrap();
    assert_eq!(c.layout.leds.len(), 1);
    m.frame(&[Rgb { r: 255, g: 2, b: 3 }], &mut h).unwrap();
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(
        h.sent,
        vec![
            report::profile_zero().to_vec(),
            report::apply().to_vec(),
            report::color(255, 2, 3).to_vec(),
            report::apply().to_vec()
        ]
    );
}
#[test]
fn all_layouts_are_valid() {
    let layout = LedLayout {
        width: 1,
        height: 1,
        leds: vec![Led {
            name: "Mouse".into(),
            pos: (0, 0),
        }],
    };
    layout.validate().unwrap();
}
