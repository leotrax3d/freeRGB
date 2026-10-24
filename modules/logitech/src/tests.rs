// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
use freergb_core::Rgb;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{InterfaceInfo, ModuleError};
#[derive(Default)]
struct Fake {
    w: Vec<Vec<u8>>,
}
struct HidppFake {
    w: Vec<Vec<u8>>,
    reads: Vec<Vec<u8>>,
}
impl Hid for HidppFake {
    fn write(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.w.push(p.to_vec());
        Ok(p.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        Ok(self.reads.remove(0))
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}
impl Hid for Fake {
    fn write(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.w.push(p.to_vec());
        Ok(p.len())
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
fn inventory() {
    assert_eq!(devices_table::DETECTORS.len(), 55)
}
#[test]
fn bytes() {
    assert_eq!(
        &report::direct(&[Rgb { r: 1, g: 2, b: 3 }])[..4],
        &[2, 1, 2, 3]
    );
    assert_eq!(
        &report::g213(2, Rgb { r: 1, g: 2, b: 3 })[..10],
        &[0x11, 0xff, 0x0c, 0x3a, 2, 1, 1, 2, 3, 2]
    );
    assert_eq!(
        &report::g203(&[Rgb { r: 1, g: 2, b: 3 }])[..9],
        &[0x11, 0xff, 0x12, 0x10, 1, 1, 2, 3, 0]
    );
    assert_eq!(
        &report::g203l_init()[..7],
        &[0x11, 0xff, 0x0e, 0x50, 1, 3, 7]
    );
    assert_eq!(
        &report::g203l(&[
            Rgb { r: 1, g: 2, b: 3 },
            Rgb { r: 4, g: 5, b: 6 },
            Rgb { r: 7, g: 8, b: 9 },
        ])[..17],
        &[0x11, 0xff, 0x12, 0x10, 1, 1, 2, 3, 2, 4, 5, 6, 3, 7, 8, 9, 0xff]
    );
    assert_eq!(
        &report::g815_little(0x04, Rgb { r: 1, g: 2, b: 3 })[..8],
        &[0x11, 0xff, 0x10, 0x1f, 4, 1, 2, 3]
    );
    assert_eq!(
        &report::lightsync(0x3a, 0, Rgb { r: 1, g: 2, b: 3 })[..10],
        &[0x11, 1, 0x0e, 0x3a, 0, 1, 1, 2, 3, 2]
    );
    assert_eq!(
        &report::x56_color(Rgb { r: 1, g: 2, b: 3 })[..6],
        &[9, 0, 3, 1, 2, 3]
    );
    assert_eq!(
        &report::g933(1, Rgb { r: 1, g: 2, b: 3 })[..10],
        &[0x11, 0xff, 4, 0x3a, 1, 1, 1, 2, 3, 2]
    );
    assert_eq!(
        report::g600(0, 3, Rgb { r: 1, g: 2, b: 3 }),
        [0xf1, 1, 2, 3, 0, 3, 0, 0]
    );
    assert_eq!(
        &report::hidpp(4, 0x10, &[1, 2, 3, 4])[..8],
        &[0x11, 2, 4, 0x17, 1, 2, 3, 4]
    );
    assert_eq!(
        &report::lightspeed_effect(5, false, 2, Rgb { r: 1, g: 2, b: 3 })[..17],
        &[0x11, 2, 5, 0x17, 2, 1, 1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 1]
    );
    assert_eq!(
        report::headset_zone(Rgb { r: 1, g: 2, b: 3 }, 4),
        [1, 2, 3, 1, 4]
    );
    assert_eq!(report::headset_end(), [1, 0, 0, 0]);
    assert_eq!(report::g815_key(0), 0x01);
    assert_eq!(report::g815_key(116), 0xb8);
    assert_eq!(report::g915_key(95), 0x50);
    assert_eq!(&report::g915_init(false)[0][..4], &[0x11, 1, 0x0f, 0x3e]);
    assert_eq!(&report::g915_commit(true)[..4], &[0x11, 0xff, 0x0a, 0x7f]);
    assert_eq!(
        &report::g915_little(false, 4, Rgb { r: 1, g: 2, b: 3 })[..8],
        &[0x11, 1, 0x0b, 0x1f, 4, 1, 2, 3]
    );
}
#[test]
fn lifecycle() {
    let d = &devices_table::DETECTORS[0];
    let i = InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let mut m = Logitech::default();
    let mut h = Fake::default();
    let c = m.initialize(&i, &mut h).unwrap();
    m.frame(&vec![Rgb { r: 1, g: 2, b: 3 }; c.layout.leds.len()], &mut h)
        .unwrap();
    assert!(!h.w.is_empty())
}
#[test]
fn hidpp_discovery_and_frame_are_brokered() {
    let d = devices_table::DETECTORS
        .iter()
        .find(|x| x.handler == "DetectLogitechHIDPP20")
        .unwrap();
    let i = InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: 0,
        usage_page: 0xff00,
        usage: 2,
        product: None,
    };
    let mut reads = Vec::new();
    reads.push(vec![0x11, 2, 0, 0x8f]);
    let mut root = vec![0; 20];
    root[..4].copy_from_slice(&[0x11, 2, 0, 7]);
    root[4] = 5;
    reads.push(root);
    let mut rgb_count = vec![0; 20];
    rgb_count[..4].copy_from_slice(&[0x11, 2, 5, 7]);
    rgb_count[6] = 2;
    reads.push(rgb_count);
    let mut perkey = vec![0; 20];
    perkey[..4].copy_from_slice(&[0x11, 2, 0, 7]);
    perkey[4] = 6;
    reads.push(perkey);
    for page in 0..3 {
        let mut zones = vec![0; 20];
        zones[..4].copy_from_slice(&[0x11, 2, 6, 7]);
        if page == 0 {
            zones[6] = 0b0000_0110;
        }
        reads.push(zones);
    }
    reads.push(vec![0x11, 2, 6, 0x17]);
    reads.push(vec![0x11, 2, 6, 0x77]);
    let mut h = HidppFake {
        w: Vec::new(),
        reads,
    };
    let mut m = Logitech::default();
    let c = m.initialize(&i, &mut h).unwrap();
    assert_eq!(c.layout.leds.len(), 2);
    m.frame(
        &[Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }],
        &mut h,
    )
    .unwrap();
    assert_eq!(h.w[0], report::hidpp(0, 0, &[0x06, 0x20]));
    assert_eq!(h.w.len(), 9);
}
