// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;

/// Simulated Cobra: answers firmware queries, records every feature report.
#[derive(Default)]
struct FakeCobra {
    sent: Vec<Vec<u8>>,
}

impl Hid for FakeCobra {
    fn write(&mut self, _: &[u8]) -> Result<usize, ModuleError> {
        panic!("Razer devices use feature reports only")
    }
    fn read(&mut self, _: usize, _: i32) -> Result<Vec<u8>, ModuleError> {
        panic!("Razer devices use feature reports only")
    }
    fn send_feature(&mut self, report: &[u8]) -> Result<usize, ModuleError> {
        assert_eq!(report.len(), REPORT_LEN);
        self.sent.push(report.to_vec());
        Ok(report.len())
    }
    fn get_feature(&mut self, prefix: &[u8], len: usize) -> Result<Vec<u8>, ModuleError> {
        assert_eq!((prefix, len), (&[0u8][..], REPORT_LEN));
        let mut reply = self.sent.last().unwrap().clone();
        reply[1] = report::STATUS_SUCCESS;
        reply[9] = 1;
        reply[10] = 4;
        Ok(reply)
    }
}

fn cobra() -> InterfaceInfo {
    InterfaceInfo {
        vid: 0x1532,
        pid: 0x00A3,
        interface: 0,
        usage_page: 1,
        usage: 2,
        product: None,
    }
}

#[test]
fn cobra_initialize_frame_and_shutdown_send_upstream_sequence() {
    let mut module = RazerChroma::default();
    let mut hid = FakeCobra::default();
    let caps = module.initialize(&cobra(), &mut hid).unwrap();
    assert_eq!(caps.name, "Razer Cobra");
    assert_eq!(caps.firmware.as_deref(), Some("v1.4"));
    assert_eq!(caps.layout.leds.len(), 1);
    assert_eq!(caps.layout.leds[0].name, "Logo");
    // get firmware (00:81), brightness extended (0F:04, led 05, 255)
    assert_eq!(hid.sent[0][6..9], [0x02, 0x00, 0x81]);
    assert_eq!(hid.sent[1][6..12], [0x03, 0x0F, 0x04, 0x00, 0x05, 0xFF]);
    hid.sent.clear();
    module
        .frame(
            &[Rgb {
                r: 255,
                g: 0,
                b: 10,
            }],
            &mut hid,
        )
        .unwrap();
    assert_eq!(hid.sent.len(), 2);
    assert_eq!(hid.sent[0][2], 0x1F, "transaction id");
    assert_eq!(
        hid.sent[0][6..17],
        [0x08, 0x0F, 0x03, 0, 0, 0, 0, 0, 255, 0, 10]
    );
    assert_eq!(hid.sent[1][6..12], [0x0C, 0x0F, 0x02, 0x00, 0x00, 0x08]);
    for r in &hid.sent {
        assert_eq!(r[89], r[3..89].iter().fold(0, |a, b| a ^ b));
    }
    hid.sent.clear();
    module.shutdown(None, &mut hid).unwrap();
    assert!(hid.sent.is_empty(), "upstream sends nothing on shutdown");
}

#[test]
fn unsupported_device_fails_initialize_without_io() {
    let mut module = RazerChroma::default();
    let mut hid = FakeCobra::default();
    let mut interface = cobra();
    interface.pid = 0xFFFF;
    let error = module.initialize(&interface, &mut hid).unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(hid.sent.is_empty());
}

#[test]
fn every_supported_layout_is_valid_and_matches_the_device_matrix() {
    for device in devices_table::DEVICES
        .iter()
        .filter(|d| d.unsupported_reason().is_none())
    {
        let (layout, _) = layout(device);
        layout
            .validate()
            .unwrap_or_else(|e| panic!("{}: {e}", device.name));
        assert_eq!(
            layout.leds.len(),
            (device.rows * device.cols) as usize,
            "{}",
            device.name
        );
    }
}
