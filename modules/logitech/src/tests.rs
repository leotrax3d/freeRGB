// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
use freergb_core::Rgb;
use freergb_module_api::sdk::Hid;
use freergb_module_api::{InterfaceInfo, ModuleError};
#[derive(Default)]
struct Fake {
    w: Vec<Vec<u8>>,
}
#[allow(dead_code)]
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
        // upstream: LogitechG600Controller.cpp SetMode, Direct mode has speed 0
        report::g600(0, 0, Rgb { r: 1, g: 2, b: 3 }),
        [0xf1, 1, 2, 3, 0, 0, 0, 0]
    );
    assert_eq!(
        // upstream: LogitechG933Controller.cpp SetDirectMode
        &report::g933_direct(1)[..5],
        &[0x11, 0xff, 4, 0xca, 1]
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
fn hidpp20_device_is_refused_without_io() {
    // The HIDPP20 protocol path (wildcard PID detectors) was audited as not
    // fully ported, so initialize must fail closed without touching the device.
    let d = devices_table::DETECTORS
        .iter()
        .find(|x| x.handler == "DetectLogitechHIDPP20")
        .unwrap();
    let i = InterfaceInfo {
        vid: d.vid,
        pid: 0x046d,
        interface: 0,
        usage_page: 0xff00,
        usage: 2,
        product: None,
    };
    let mut m = Logitech::default();
    let mut h = Fake::default();
    let res = m.initialize(&i, &mut h);
    assert!(res.is_err());
    assert!(h.w.is_empty());
}

#[test]
fn unported_handler_is_reported_not_guessed() {
    // upstream: LogitechControllerDetect.cpp registers HID++ wildcard rows with
    // HID_PID_ANY; they are inventoried but their protocol path is not ported.
    let d = devices_table::DETECTORS
        .iter()
        .find(|x| x.handler == "DetectLogitechHIDPP20")
        .unwrap();
    let i = InterfaceInfo {
        vid: d.vid,
        pid: 0x1234, // wildcard never matches an actual VID/PID
        interface: 0,
        usage_page: 0xff00,
        usage: 2,
        product: None,
    };
    assert!(!table::identify(&i).supported);
    let d2 = devices_table::DETECTORS
        .iter()
        .find(|x| x.handler == "DetectLogitechLightspeedReceiver")
        .unwrap();
    let i = InterfaceInfo {
        vid: d2.vid,
        pid: d2.pid,
        interface: 0,
        usage_page: 0,
        usage: 0,
        product: None,
    };
    let id = table::identify(&i);
    assert!(!id.supported);
    assert!(id.reason.as_deref().unwrap().contains("not fully ported"));
}

#[test]
fn g933_initialize_sends_direct_mode_packets() {
    let d = devices_table::DETECTORS
        .iter()
        .find(|x| x.handler == "DetectLogitechG933")
        .unwrap();
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
    assert_eq!(c.layout.leds.len(), 2);
    // upstream: RGBController_LogitechG933.cpp DeviceUpdateMode for direct mode
    assert_eq!(h.w[0], report::g933_direct(0));
    assert_eq!(h.w[1], report::g933_direct(1));
}
