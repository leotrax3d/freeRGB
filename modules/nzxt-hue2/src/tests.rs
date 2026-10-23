// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: NZXTHue2ControllerDetect.cpp registers 21 detectors; all route
    // into RGBController_NZXTHue2 via spawn_hue.
    assert_eq!(crate::devices_table::DETECTORS.len(), 21);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x1e71 && d.channels > 0));
    // upstream: spawn_hue(info, name, 3, 0) for Hue 2 Motherboard, (3, 0) for
    // Kraken X3 and RGB Controller, (4, 0) for Hue 2, (6, 3) for 6-channel.
    let c = |p: u16| {
        crate::devices_table::DETECTORS
            .iter()
            .find(|d| d.pid == p)
            .unwrap()
            .channels
    };
    assert_eq!(c(0x2005), 2);
    assert_eq!(c(0x2007), 3);
    assert_eq!(c(0x2001), 4);
    assert_eq!(c(0x2011), 6);
    assert_eq!(c(0x2012), 3);
}

fn info_for(row: usize) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[row];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

struct Fake {
    writes: Vec<Vec<u8>>,
    reads: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, len: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        let r = self.reads.remove(0);
        Ok(r.into_iter().take(len).collect())
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    // Hue 2 (0x2001), 4 channels from spawn_hue(4, 0); one 8-LED fan on channel 0.
    let row = crate::devices_table::DETECTORS
        .iter()
        .position(|d| d.handler == "DetectNZXTHue2")
        .unwrap();
    let mut reply = vec![0; 64];
    reply[0] = 0x21;
    reply[1] = 0x03;
    reply[0x0f] = 0x02; // Aer 1 fan -> 8 LEDs
    let mut m = crate::Nzxt::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![reply],
    };
    let caps = m.initialize(&info_for(row), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 8);
    assert_eq!(caps.zones.len(), 4);
    // query + 3 empty channel writes are not sent: only the request
    assert_eq!(h.writes.len(), 1);
    assert_eq!(h.writes[0][..2], [0x20, 0x03]);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 8];
    m.frame(&colors, &mut h).unwrap();
    // every channel gets direct(+apply); channel 0 has an Aer 1 fan (8 LEDs)
    assert_eq!(h.writes.len(), 9);
    let d0 = &h.writes[1];
    assert_eq!(d0[..3], [0x22, 0x10, 1]);
    assert_eq!(&d0[4..13], &[2, 1, 3, 2, 1, 3, 2, 1, 3]);
    assert_eq!(h.writes[2][..3], [0x22, 0xa0, 1]);
    // empty channels: direct with zero colors + apply
    assert_eq!(h.writes[3][..4], [0x22, 0x10, 2, 0]);
    assert_eq!(h.writes[4][..3], [0x22, 0xa0, 2]);
    assert_eq!(&h.writes[5][..3], &[0x22, 0x10, 4]);
    assert_eq!(h.writes[6][..3], [0x22, 0xa0, 4]);
    assert_eq!(&h.writes[7][..3], &[0x22, 0x10, 8]);
    assert_eq!(h.writes[8][..3], [0x22, 0xa0, 8]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 9); // no exit packets
}

#[test]
fn unknown_device_is_not_supported_and_kraken_is_supported() {
    let kraken = crate::devices_table::DETECTORS
        .iter()
        .position(|d| d.pid == 0x2007)
        .unwrap();
    assert!(crate::table::identify(&info_for(kraken)).supported);
    let mut m = crate::Nzxt::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![],
    };
    let mut i = info_for(kraken);
    i.vid = 0x1234;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    assert!(h.writes.is_empty());
}

#[test]
fn device_list_reads_are_bounded() {
    // a non-21 03 reply must eventually fail closed (max retries like upstream)
    let row = crate::devices_table::DETECTORS
        .iter()
        .position(|d| d.handler == "DetectNZXTHue2")
        .unwrap();
    let mut m = crate::Nzxt::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![vec![0; 64]; 16],
    };
    assert!(m.initialize(&info_for(row), &mut h).is_err());
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let row = crate::devices_table::DETECTORS
        .iter()
        .position(|d| d.handler == "DetectNZXTHue2")
        .unwrap();
    let mut reply = vec![0; 64];
    reply[0] = 0x21;
    reply[1] = 0x03;
    reply[0x0f] = 0x02;
    let mut m = crate::Nzxt::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![reply],
    };
    let caps = m.initialize(&info_for(row), &mut h).unwrap();
    let sum: usize = caps.zones.iter().map(|z| z.leds).sum();
    assert_eq!(caps.layout.leds.len(), sum);
    let colors = vec![Rgb { r: 0, g: 0, b: 0 }; sum];
    assert!(m.frame(&colors, &mut h).is_ok());
    assert!(m.frame(&colors[..sum - 1], &mut h).is_err());
}
