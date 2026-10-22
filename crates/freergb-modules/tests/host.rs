//! Real child processes: lifecycle, brokered reports, crash/hang isolation,
//! restart limit and unplug handling.
use freergb_core::{Frame, Rgb};
use freergb_device_api::{DeviceError, DeviceId, HidTransport, RgbDevice, TransportError};
use freergb_module_api::{InterfaceInfo, Zone};
use freergb_modules::host::{Launch, ModuleDevice, MAX_CONSECUTIVE_FAILURES, REQUEST_TIMEOUT};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Default)]
struct Wire {
    written: Arc<Mutex<Vec<Vec<u8>>>>,
    unplugged: Arc<Mutex<bool>>,
}

struct Echo(Wire, Option<Vec<u8>>);

impl HidTransport for Echo {
    fn write(&mut self, report: &[u8]) -> Result<usize, TransportError> {
        if *self.0.unplugged.lock().unwrap() {
            return Err(TransportError::Disconnected);
        }
        self.0.written.lock().unwrap().push(report.to_vec());
        self.1 = Some(report[1..].to_vec());
        Ok(report.len())
    }
    fn read_timeout(&mut self, buf: &mut [u8], _: i32) -> Result<usize, TransportError> {
        let data = self.1.take().unwrap_or_default();
        buf[..data.len()].copy_from_slice(&data);
        Ok(data.len())
    }
    fn output_report_len(&self) -> usize {
        64
    }
    fn input_report_len(&self) -> usize {
        64
    }
}

fn device(mode: &str, wire: &Wire) -> ModuleDevice {
    let dir = std::env::temp_dir().join(format!("freergb-host-{mode}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("fixture-mode"), mode).unwrap();
    let launch = Launch {
        module_id: "fixture".into(),
        executable: env!("CARGO_BIN_EXE_freergb-module-fixture").into(),
        directory: dir,
    };
    let interface = InterfaceInfo {
        vid: 1,
        pid: 2,
        interface: 0,
        usage_page: 1,
        usage: 2,
        product: None,
    };
    let wire = wire.clone();
    ModuleDevice::new(
        DeviceId("module:fixture:test".into()),
        "Fixture".into(),
        launch,
        interface,
        Box::new(move || Ok(Box::new(Echo(wire.clone(), None)) as Box<dyn HidTransport>)),
    )
}

const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };

#[test]
fn frames_are_bundled_brokered_and_shutdown_releases() {
    let wire = Wire::default();
    let mut d = device("ok", &wire);
    d.connect().unwrap();
    assert_eq!(d.info().name, "Fixture Mouse");
    assert_eq!(d.layout().leds.len(), 2);
    d.push_frame(&Frame::from_colors(vec![RED, Rgb { r: 1, g: 2, b: 3 }]))
        .unwrap();
    assert!(
        d.push_frame(&Frame::from_colors(vec![RED])).is_err(),
        "frame size checked by host"
    );
    d.disconnect(Some(RED)).unwrap();
    let w = wire.written.lock().unwrap().clone();
    assert_eq!(
        w,
        vec![vec![0, 0xAB], vec![1, 255, 0, 0, 1, 2, 3], vec![2, 255]],
        "one report per frame, not per LED"
    );
}

#[test]
fn optional_zone_configuration_reaches_module_and_changes_layout() {
    let wire = Wire::default();
    let mut d = device("ok", &wire);
    d.set_zones(Some(vec![
        Zone {
            name: "Header 1".into(),
            leds: 12,
        },
        Zone {
            name: "Header 2".into(),
            leds: 3,
        },
    ]));
    d.connect().unwrap();
    assert_eq!(d.layout().leds.len(), 15);
}

#[test]
fn crash_and_hang_fault_only_this_device_and_restarts_are_limited() {
    let wire = Wire::default();
    let mut crash = device("crash-frame", &wire);
    crash.connect().unwrap();
    assert!(crash.push_frame(&Frame::solid(2, RED)).is_err());
    assert!(crash.connect().is_ok(), "fresh process after a crash");
    let mut hang = device("hang-frame", &wire);
    hang.connect().unwrap();
    let start = Instant::now();
    assert!(hang.push_frame(&Frame::solid(2, RED)).is_err());
    assert!(start.elapsed() < REQUEST_TIMEOUT * 2, "deadline enforced");
    let mut refuse = device("refuse", &wire);
    for _ in 0..MAX_CONSECUTIVE_FAILURES {
        assert!(matches!(refuse.connect(), Err(DeviceError::Protocol(_))));
    }
    assert!(
        matches!(refuse.connect(), Err(DeviceError::Refused(_))),
        "gives up instead of looping"
    );
    let mut bad = device("bad-layout", &wire);
    assert!(bad.connect().is_err(), "empty layout rejected");
}

#[test]
fn unplugged_device_is_reported_gone() {
    let wire = Wire::default();
    let mut d = device("ok", &wire);
    d.connect().unwrap();
    *wire.unplugged.lock().unwrap() = true;
    let error = d.push_frame(&Frame::solid(2, RED)).unwrap_err();
    assert!(error.is_gone(), "{error}");
}
