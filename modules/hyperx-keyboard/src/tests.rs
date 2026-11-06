// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
use freergb_module_api::sdk::{Hid, Module};
use freergb_module_api::{InterfaceInfo, ModuleError};

#[test]
fn inventory_is_complete() {
    // upstream: HyperXKeyboardControllerDetect.cpp registers 20 rows (13 unique
    // devices; the Origins rows exist on interface 0 and 3).
    assert_eq!(crate::devices_table::DETECTORS.len(), 20);
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.kind == "elite")
            .count(),
        1
    );
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.kind == "fps")
            .count(),
        1
    );
    assert_eq!(
        crate::devices_table::DETECTORS
            .iter()
            .filter(|d| d.handler == "DetectHyperXAlloyOrigins")
            .count(),
        4
    );
}

fn info_for(pid: u16) -> InterfaceInfo {
    let d = crate::devices_table::DETECTORS
        .iter()
        .find(|d| d.pid == pid)
        .unwrap();
    InterfaceInfo {
        vid: d.vid,
        pid: d.pid,
        interface: d.interface as i32,
        usage_page: d.usage_page,
        usage: 0,
        product: None,
    }
}

#[derive(Default)]
struct Fake {
    features: Vec<Vec<u8>>,
    writes: Vec<Vec<u8>>,
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
fn supported_devices_match_the_audit() {
    // Only Alloy Elite RGB (0x16BE) and Alloy FPS RGB (0x16DC) have byte-exact
    // Direct-mode paths (channel packets + Elite extended packet).
    let elite = crate::table::identify(&info_for(0x16be));
    assert!(elite.supported);
    let fps = crate::table::identify(&info_for(0x16dc));
    assert!(fps.supported);
    // unsupported: Origins (265 skip insertion + wrong counts), Elite2 (fabricated
    // init), Origins2/Eve (missing init), Origins Core (segment/sector layout),
    // Origins 60/65 (wrong counts 61/67 vs 71/77).
    for pid in [
        0x16e5u16, 0x0591, 0x1734, 0x0c8e, 0x038f, 0x1711, 0x058f, 0x0cc2, 0x08c2, 0x16e6, 0x098f,
    ] {
        let id = crate::table::identify(&info_for(pid));
        assert!(!id.supported, "pid {pid} must not claim support");
        assert!(id.reason.as_deref().unwrap().contains("not fully ported"));
    }
}

#[test]
fn elite_and_fps_frames_are_feature_reports_with_upstream_packets() {
    let mut m = crate::HyperX::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x16dc), &mut h).unwrap();
    // upstream: RGBController_HyperXAlloyFPS.cpp SetupZones: one 106-LED zone
    assert_eq!(caps.layout.leds.len(), 106);
    assert_eq!(caps.layout.width, 23);
    assert_eq!(caps.layout.height, 6);
    assert!(h.features.is_empty()); // no init packets upstream
    let colors = vec![Rgb { r: 1, g: 2, b: 3 }; 106];
    m.frame(&colors, &mut h).unwrap();
    // three 264-byte channel feature reports
    assert_eq!(h.features.len(), 3);
    assert!(h.writes.is_empty());
    assert_eq!(&h.features[0][..5], &[7, 0x16, 1, 0xa0, 0]);
    // first keyboard LED from the upstream KEYS table gets the R plane value
    assert_eq!(h.features[0][0x08], 1);
    assert_eq!(h.features[1][0x08], 2);
    assert_eq!(h.features[2][0x08], 3);
    m.shutdown(None, &mut h).unwrap();
    assert_eq!(h.features.len(), 3); // upstream sends no exit packets
}

#[test]
fn elite_extended_packet_covers_media_leds() {
    let mut m = crate::HyperX::default();
    let mut h = Fake::default();
    let caps = m.initialize(&info_for(0x16be), &mut h).unwrap();
    // upstream: RGBController_HyperXAlloyElite.cpp zones 106 + 18 + 4
    assert_eq!(caps.layout.leds.len(), 128);
    let mut colors = vec![Rgb { r: 0, g: 0, b: 0 }; 128];
    colors[106] = Rgb { r: 9, g: 8, b: 7 };
    m.frame(&colors, &mut h).unwrap();
    assert_eq!(h.features.len(), 4);
    let ext = &h.features[3];
    assert_eq!(&ext[..4], &[7, 0x16, 4, 0xa0]);
    // upstream: extended_red[0] = 0x08 carries the first media LED's red value
    assert_eq!(ext[0x08], 9);
}

#[test]
fn wrong_interface_is_not_supported() {
    let mut i = info_for(0x16dc);
    i.interface = 0; // upstream registers interface 2 with usage page 0xFF01
    assert!(!crate::table::identify(&i).supported);
}

#[test]
fn layouts_are_valid_and_frame_size_is_checked() {
    for kind_pid in [0x16beu16, 0x16dc] {
        let mut m = crate::HyperX::default();
        let mut h = Fake::default();
        let caps = m.initialize(&info_for(kind_pid), &mut h).unwrap();
        let led_count = caps.layout.leds.len();
        for led in caps.layout.leds.iter() {
            assert!(led.pos.0 < caps.layout.width && led.pos.1 < caps.layout.height);
        }
        let bad = vec![Rgb { r: 0, g: 0, b: 0 }; led_count.saturating_sub(1).max(1)];
        assert!(m.frame(&bad, &mut h).is_err());
    }
}
