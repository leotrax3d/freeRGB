// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

struct RecordingWrite {
    writes: Vec<Vec<u8>>,
}
impl Hid for RecordingWrite {
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

/// upstream: CryorigH7QuadLumiController.cpp SetChannelLEDs/SendPacket direct mode
#[test]
fn report_direct_is_byte_exact() {
    use crate::report;
    let colors = [
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
    ];
    let p = report::direct(0, &colors);
    assert_eq!(p.len(), 65);
    assert_eq!(p[0], 0x02);
    assert_eq!(p[1], 0x4C);
    assert_eq!(p[2], 0x01);
    assert_eq!(p[3], 0x00);
    // GRB swap: first color G,R,B at offset 5
    assert_eq!(&p[5..8], &[2, 1, 3]);
    // channel slot for channel 0
    assert_eq!(p[5 + 3], 5); // second LED G
    let p1 = report::direct(1, &colors);
    assert_eq!(p1[2], 0x02);
    // data for channel 1 starts at 5+15=20
    assert_eq!(&p1[20..23], &[2, 1, 3]);
    // channel 0 slot zeroed in channel-1 packet (offset 5..20 zeros)
    assert!(p1[5..20].iter().all(|&b| b == 0));
}

#[test]
fn inventory_is_complete() {
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
    assert_eq!(crate::devices_table::DETECTORS[0].vid, 0x1E71);
    assert_eq!(crate::devices_table::DETECTORS[0].pid, 0x1712);
}

fn supported_interface() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1E71,
        pid: 0x1712,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    }
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::CryorigH7QuadLumi::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 10);
    assert!(h.writes.is_empty());
    let colors: Vec<Rgb> = (0..10)
        .map(|i| Rgb {
            r: (i as u8) * 3,
            g: (i as u8) * 5,
            b: (i as u8) * 7,
        })
        .collect();
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2);
    // reconstruct expected packets
    let mut ch0 = [Rgb::default(); 5];
    ch0.copy_from_slice(&colors[0..5]);
    let mut ch1 = [Rgb::default(); 5];
    ch1.copy_from_slice(&colors[5..10]);
    assert_eq!(h.writes[0], crate::report::direct(0, &ch0));
    assert_eq!(h.writes[1], crate::report::direct(1, &ch1));
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2);
}

#[test]
fn wrong_device_is_not_supported() {
    let mut i = supported_interface();
    i.pid = 0x0000;
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn frame_size_is_checked_and_layouts_are_valid() {
    let mut m = crate::CryorigH7QuadLumi::default();
    let mut h = RecordingWrite { writes: Vec::new() };
    let caps = m.initialize(&supported_interface(), &mut h).unwrap();
    assert_eq!(caps.layout.width, 10);
    assert_eq!(caps.layout.height, 1);
    for (idx, led) in caps.layout.leds.iter().enumerate() {
        assert!(led.pos.0 < caps.layout.width);
        assert_eq!(led.pos.0, idx as u16);
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 9], &mut h).is_err());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 10], &mut h).is_ok());
}
