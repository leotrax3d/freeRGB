// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;

#[derive(Default)]
struct Fake {
    feature: Vec<Vec<u8>>,
    writes: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(p.to_vec());
        Ok(p.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        Ok(Vec::new())
    }
    fn send_feature(&mut self, p: &[u8]) -> Result<usize, ModuleError> {
        self.feature.push(p.to_vec());
        Ok(p.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        Ok(Vec::new())
    }
}
fn iface(pid: u16) -> InterfaceInfo {
    iface_vid(0x1e7d, pid)
}
fn iface_vid(vid: u16, pid: u16) -> InterfaceInfo {
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
fn generated_inventory_covers_all_detectors() {
    assert_eq!(devices_table::DETECTORS.len(), 28);
    assert_eq!(devices_table::DEVICES.len(), 28);
    for d in devices_table::DEVICES {
        assert!(table::identify(&iface_vid(d.vid, d.pid)).supported);
    }
}

#[test]
fn kova_lifecycle_uses_feature_reports() {
    let mut m = Roccat::default();
    let mut h = Fake::default();
    let caps = m.initialize(&iface(0x2cee), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 2);
    m.frame(
        &[Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }],
        &mut h,
    )
    .unwrap();
    assert_eq!(h.feature[0], report::kova_init());
    assert_eq!(
        h.feature[1],
        report::kova(&[Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 4, g: 5, b: 6 }])
    );
}

#[test]
fn vulcan_frame_uses_openrgb_chunk_headers() {
    let mut m = Roccat::default();
    let mut h = Fake::default();
    m.initialize(&iface(0x307a), &mut h).unwrap();
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 105];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.feature[0], report::keyboard_direct_mode(false));
    assert_eq!(h.writes.len(), 7);
    assert_eq!(&h.writes[0][1..4], &[0xa1, 1, 180]);
}
