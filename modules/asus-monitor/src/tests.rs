// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: AsusMonitorControllerDetect.cpp registers 7 IPU rows
    // (6 monitors on interface 1, the light bar on interface 2, usage 0x00A1).
    assert_eq!(crate::devices_table::DETECTORS.len(), 7);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x0b05 && d.usage_page == 0xff72 && d.usage == 0x00a1));
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.interface == 2)
            .count(),
        1
    );
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid,
        interface: d.interface as i32,
        usage_page: d.usage_page,
        usage: d.usage,
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
        if self.reads.is_empty() {
            return Err(ModuleError::new(
                freergb_module_api::ErrorCode::Disconnected,
                "no queued reads",
            ));
        }
        Ok(self.reads.remove(0).into_iter().take(len).collect())
    }
    fn send_feature(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        unreachable!()
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn packets_are_byte_exact() {
    // upstream: AsusMonitorController.cpp GetNumberOfLEDs / SendInit / SetDirect
    assert_eq!(&crate::report::query()[0..2], &[0xec, 0xb0]);
    assert_eq!(
        &crate::report::init()[0..9],
        &[0xec, 0x35, 0, 0, 0, 0xff, 0, 0, 1]
    );
    assert_eq!(
        &crate::report::direct(&[Rgb { r: 1, g: 2, b: 3 }])[0..9],
        &[0xec, 0x40, 0x84, 0, 1, 1, 2, 3, 0]
    );
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![vec![
            0xec, 0xb1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 4,
        ]],
    };
    let caps = m.initialize(&info_for(0x1ba3), &mut h).unwrap();
    // count from response byte 32
    assert_eq!(caps.layout.leds.len(), 4);
    // query + init
    assert_eq!(h.writes[0][..2], [0xec, 0xb0]);
    assert_eq!(h.writes[1][..2], [0xec, 0x35]);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 4];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 3);
    let f = &h.writes[2];
    assert_eq!(f[..2], [0xec, 0x40]);
    assert_eq!(f[2], 0x84);
    assert_eq!(f[4], 4);
    assert_eq!(&f[5..14], &[1, 2, 3, 1, 2, 3, 1, 2, 3]);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 3); // upstream sends no exit packets
}

#[test]
fn missing_led_count_fails_closed() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![],
    };
    assert!(m.initialize(&info_for(0x1ba3), &mut h).is_err());
    // zero count: upstream would yield 0 LEDs, the module refuses
    h.writes.clear();
    h.reads = vec![vec![
        0xec, 0xb1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0,
    ]];
    assert!(m.initialize(&info_for(0x1ba3), &mut h).is_err());
}

#[test]
fn wrong_interface_or_usage_is_not_supported() {
    let mut i = info_for(0x1ba3);
    i.interface = 0; // upstream registers interface 1
    assert!(!crate::table::identify(&i).supported);
    i = info_for(0x1ba3);
    i.usage = 0x0001;
    assert!(!crate::table::identify(&i).supported);
    // unknown PID
    assert!(!crate::table::identify(&info_for(0x9999)).supported);
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake {
        writes: Vec::new(),
        reads: vec![vec![
            0xec, 0xb1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 6,
        ]],
    };
    let caps = m.initialize(&info_for(0x1b2b), &mut h).unwrap();
    for (n, led) in caps.layout.leds.iter().enumerate() {
        assert_eq!(led.pos.0, n as u16);
        assert_eq!(led.name, format!("LED {}", n));
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 6], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 5], &mut h).is_err());
}
