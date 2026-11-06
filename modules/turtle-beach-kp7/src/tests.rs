// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: TurtleBeachKP7ControllerDetect.cpp registers one
    // REGISTER_HID_DETECTOR_I row (0x10F5:0x503A, interface 4).
    assert_eq!(crate::devices_table::DETECTORS.len(), 1);
    assert!(crate::devices_table::DETECTORS
        .iter()
        .all(|d| d.vid == 0x10f5 && d.pid == 0x503a && d.interface == 4));
}

fn info_for() -> InterfaceInfo {
    let d = &crate::devices_table::DETECTORS[0];
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: i32::from(d.interface),
        usage_page: d.usage_page,
        usage: d.usage,
        product: None,
    }
}

#[derive(Default)]
struct Fake {
    writes: Vec<Vec<u8>>,
    features: Vec<Vec<u8>>,
}
impl Hid for Fake {
    fn write(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.writes.push(b.to_vec());
        Ok(b.len())
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
    fn send_feature(&mut self, b: &[u8]) -> Result<usize, ModuleError> {
        self.features.push(b.to_vec());
        Ok(b.len())
    }
    fn get_feature(&mut self, _: &[u8], _: usize) -> Result<Vec<u8>, ModuleError> {
        unreachable!()
    }
}

#[test]
fn packets_are_byte_exact() {
    // upstream: TurtleBeachKP7Controller.cpp EnterDirectMode
    assert_eq!(
        crate::report::enter_direct(),
        vec![14u8, 0x00, 0x01, 0x00, 0x00]
    );
    // upstream: RGBController_TurtleBeachKP7.cpp DeviceUpdateLEDs +
    // TurtleBeachKP7Controller.cpp SendFrame. Zone LED 0 maps to device
    // slot 28 => buffer bytes 84..87, which are past the 62-byte page 0
    // split, so both colors land in the page 1 report.
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 33];
    colors[0] = Rgb { r: 1, g: 2, b: 3 };
    colors[1] = Rgb { r: 4, g: 5, b: 6 };
    let packets = crate::report::frame(&colors);
    assert_eq!(packets.len(), 2);
    let p0 = &packets[0];
    assert_eq!(p0[0], 0xef);
    assert_eq!(p0[1], 0x00);
    assert_eq!(p0.len(), 64);
    // page 0 carries buffer bytes 0..61; slots at those indexes are unset
    assert!(&p0[2..].iter().all(|b| *b == 0));
    let p1 = &packets[1];
    assert_eq!(p1[0], 0xef);
    assert_eq!(p1[1], 0x01);
    assert_eq!(p1.len(), 64);
    // buffer bytes 84..86 (slot 28) land at report offset 2 + (84 - 62) = 24
    assert_eq!(&p1[24..27], &[1, 2, 3]);
    // buffer bytes 87..89 (slot 29) land at report offset 27
    assert_eq!(&p1[27..30], &[4, 5, 6]);
    // everything past buffer byte 99 is zero padded
    assert!(&p1[33..].iter().all(|b| *b == 0));
}

#[test]
fn initialize_frame_shutdown_sequence_is_byte_exact() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    assert_eq!(caps.layout.leds.len(), 33);
    assert_eq!(caps.layout.leds[0].name, "Num Lock Indicator");
    assert_eq!(caps.layout.leds[30].name, "Space");
    assert_eq!(caps.layout.leds[31].name, "Wrist Rest 1");
    assert_eq!(caps.zones.len(), 3);
    assert_eq!(
        (caps.zones[0].leds, caps.zones[1].leds, caps.zones[2].leds),
        (3, 28, 2)
    );
    // one enter-direct feature report at init
    assert_eq!(h.features, vec![vec![14u8, 0, 1, 0, 0]]);
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 33];
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2);
    assert_eq!(h.writes[0][0], 0xef);
    assert_eq!(h.writes[0][1], 0x00);
    assert_eq!(h.writes[1][0], 0xef);
    assert_eq!(h.writes[1][1], 0x01);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.writes.len(), 2); // upstream sends no exit packets
    assert!(m.frame(&colors, &mut h).is_err());
}

#[test]
fn unsupported_device_is_rejected() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let mut i = info_for();
    i.interface = 0;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
    let mut i = info_for();
    i.pid = 0x503e;
    assert!(!crate::table::identify(&i).supported);
    assert!(m.initialize(&i, &mut h).is_err());
}

#[test]
fn layouts_are_valid() {
    let mut m = crate::M::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(), &mut h).unwrap();
    for led in &caps.layout.leds {
        assert!(led.pos.0 < caps.layout.width);
        assert!(led.pos.1 < caps.layout.height);
        assert!(!led.name.is_empty());
    }
    // every keypad LED has a unique position
    for (a, x) in caps.layout.leds.iter().enumerate() {
        for y in caps.layout.leds.iter().skip(a + 1) {
            assert!(x.pos != y.pos, "duplicate position for LED {}", x.name);
        }
    }
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 33], &mut h).is_ok());
    assert!(m.frame(&[Rgb { r: 0, g: 0, b: 0 }; 32], &mut h).is_err());
}
