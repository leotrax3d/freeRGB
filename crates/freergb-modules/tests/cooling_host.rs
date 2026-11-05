//! Real cooling plugin child processes: crash, hang, malformed output, policy
//! violations, lease expiry, floor/pump rules and the host-side fail-safe.
//! The "device" is an in-memory recorder; nothing touches hardware.
use freergb_device_api::{HidTransport, TransportError};
use freergb_module_api::cooling::*;
use freergb_module_api::Op;
use freergb_modules::cooling_host::CoolingPlugin;
use freergb_modules::package::PackageManifest;
use freergb_modules::store::{Verified, VerifiedCooling};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct Wire(Arc<Mutex<Vec<Vec<u8>>>>);

impl Wire {
    fn writes(&self) -> Vec<Vec<u8>> {
        self.0.lock().unwrap().clone()
    }
}

impl HidTransport for Wire {
    fn write(&mut self, report: &[u8]) -> Result<usize, TransportError> {
        self.0.lock().unwrap().push(report.to_vec());
        Ok(report.len())
    }
    fn read_timeout(&mut self, _: &mut [u8], _: i32) -> Result<usize, TransportError> {
        Ok(0)
    }
    fn output_report_len(&self) -> usize {
        64
    }
    fn input_report_len(&self) -> usize {
        64
    }
}

const FAILSAFE: [u8; 2] = [0x03, 0xff];

fn policy() -> Policy {
    let rule = |prefix: &[u8], max_len| HidRule {
        op: Op::Write,
        prefix: prefix.to_vec(),
        max_len,
    };
    Policy {
        capability: Capability::Both,
        sensors: vec![SensorDecl {
            id: "rpm".into(),
            name: "Fan 1".into(),
            unit: Unit::Rpm,
            min: 0.0,
            max: 5000.0,
        }],
        channels: vec![
            ChannelDecl {
                id: "fan1".into(),
                name: "Fan 1".into(),
                role: Role::Fan,
                floor_percent: 40,
                tachometer: Some("rpm".into()),
            },
            ChannelDecl {
                id: "pump".into(),
                name: "Pump".into(),
                role: Role::Pump,
                floor_percent: 100,
                tachometer: None,
            },
        ],
        io: Io::Hid {
            devices: vec![HidMatch {
                vid: 0x1234,
                pid: 0x5678,
                interface: None,
                usage_page: None,
            }],
            rules: vec![
                rule(&[0x01], 2),
                rule(&[0x02], 2),
                rule(&[0x03], 2),
                rule(&[0x04], 1),
            ],
            failsafe: vec![FailsafeReport {
                op: Op::Write,
                data: FAILSAFE.to_vec(),
            }],
        },
        sample_interval_ms: 1000,
        max_age_ms: 3000,
    }
}

fn start(mode: &str) -> (Result<CoolingPlugin, String>, Wire) {
    let dir = std::env::temp_dir().join(format!("freergb-cooling-{mode}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("fixture-mode"), mode).unwrap();
    std::fs::write(
        dir.join("fixture-policy.json"),
        serde_json::to_vec(&policy()).unwrap(),
    )
    .unwrap();
    let sha = "0".repeat(64);
    let exe = env!("CARGO_BIN_EXE_freergb-cooling-fixture");
    let manifest = PackageManifest {
        schema: 3,
        id: "cooling-fixture".into(),
        version: "0.0.1".into(),
        family: "fixture".into(),
        api: vec![API],
        target: "test".into(),
        executable: "fixture".into(),
        license: "MIT".into(),
        devices: vec![],
        bus: None,
        cooling: Some(policy()),
        files: [
            ("fixture".to_owned(), sha.clone()),
            ("PROVENANCE.md".to_owned(), sha.clone()),
            ("LICENSE".to_owned(), sha),
        ]
        .into(),
    };
    let verified = VerifiedCooling::unpackaged(Verified {
        manifest,
        executable: exe.into(),
        directory: dir,
    })
    .unwrap();
    let wire = Wire::default();
    let plugin =
        CoolingPlugin::start(&verified, Some(Box::new(wire.clone()))).map_err(|e| e.message);
    (plugin, wire)
}

fn failsafe_written(wire: &Wire) -> bool {
    wire.writes().last() == Some(&FAILSAFE.to_vec())
}

#[test]
fn healthy_plugin_samples_controls_and_releases() {
    let (plugin, wire) = start("ok");
    let mut plugin = plugin.unwrap();
    let now = Instant::now();
    plugin.sample(now).unwrap();
    let readings = plugin.readings(now);
    assert_eq!(readings[0].value, Some(1000.0));
    plugin.set_duty("fan1", 55, now).unwrap();
    plugin.set_duty("pump", 100, now).unwrap();
    assert_eq!(plugin.status().controls.len(), 2);
    plugin.shutdown().unwrap();
    let writes = wire.writes();
    assert!(writes.contains(&vec![0x02, 55]));
    // Two releases (fan, pump) and then the plugin's own shutdown report.
    assert_eq!(writes.iter().filter(|w| *w == &vec![0x03]).count(), 2);
    assert_eq!(writes.last(), Some(&vec![0x04]));
    assert!(plugin.status().failsafe.is_none());
}

#[test]
fn pumps_floors_and_stale_tachometers_are_enforced() {
    let (plugin, wire) = start("ok");
    let mut plugin = plugin.unwrap();
    let now = Instant::now();
    assert!(
        plugin.set_duty("fan1", 50, now).is_err(),
        "no tachometer sample yet"
    );
    plugin.sample(now).unwrap();
    assert!(plugin
        .set_duty("pump", 99, now)
        .unwrap_err()
        .contains("never reduced"));
    assert!(plugin.set_duty("fan1", 39, now).is_err(), "below floor");
    assert!(plugin.set_duty("ghost", 100, now).is_err());
    assert!(!wire.writes().iter().any(|w| w[0] == 0x02));
    plugin.set_duty("fan1", 60, now).unwrap();
    // The tachometer ages out: the reduction falls back to 100 % and release.
    let later = now + Duration::from_millis(3500);
    assert_eq!(plugin.readings(later)[0].value, None);
    assert_eq!(plugin.poll(later), vec!["fan1".to_owned()]);
    let writes = wire.writes();
    assert_eq!(&writes[writes.len() - 2..], &[vec![0x02, 100], vec![0x03]]);
}

#[test]
fn expired_lease_falls_back_to_full_then_firmware() {
    let (plugin, wire) = start("ok");
    let mut plugin = plugin.unwrap();
    let now = Instant::now();
    plugin.sample(now).unwrap();
    plugin.set_duty("fan1", 45, now).unwrap();
    assert!(plugin.poll(now + Duration::from_millis(1000)).is_empty());
    plugin.sample(now + Duration::from_millis(1900)).unwrap();
    // No renewal for LEASE_MS.
    let expired = now + Duration::from_millis(LEASE_MS + 10);
    assert_eq!(plugin.poll(expired), vec!["fan1".to_owned()]);
    let writes = wire.writes();
    assert_eq!(&writes[writes.len() - 2..], &[vec![0x02, 100], vec![0x03]]);
    assert!(plugin.status().controls.is_empty());
    assert!(plugin.is_running());
}

#[test]
fn crash_runs_host_failsafe() {
    for mode in ["crash-sample", "crash-set"] {
        let (plugin, wire) = start(mode);
        let mut plugin = plugin.unwrap();
        let now = Instant::now();
        if mode == "crash-set" {
            plugin.sample(now).unwrap();
            assert!(plugin.set_duty("fan1", 50, now).is_err());
        } else {
            assert!(plugin.sample(now).is_err());
        }
        let status = plugin.status();
        assert!(!status.running && status.fault.is_some(), "{mode}");
        assert_eq!(status.failsafe.unwrap().written, 1);
        assert!(failsafe_written(&wire), "{mode}");
        assert!(plugin.set_duty("fan1", 100, now).is_err(), "stays faulted");
    }
}

#[test]
fn hang_hits_deadline_and_runs_failsafe() {
    let (plugin, wire) = start("hang-sample");
    let mut plugin = plugin.unwrap();
    let started = Instant::now();
    let error = plugin.sample(started).unwrap_err();
    assert!(error.message.contains("in time"), "{}", error.message);
    assert!(
        started.elapsed() < freergb_modules::cooling_host::SAMPLE_TIMEOUT + Duration::from_secs(2)
    );
    assert!(failsafe_written(&wire));
}

#[test]
fn malformed_output_faults() {
    for mode in ["out-of-range", "undeclared", "garbage"] {
        let (plugin, wire) = start(mode);
        let mut plugin = plugin.unwrap();
        assert!(plugin.sample(Instant::now()).is_err(), "{mode}");
        assert!(plugin.status().fault.is_some(), "{mode}");
        assert!(failsafe_written(&wire), "{mode}");
        assert_eq!(plugin.readings(Instant::now())[0].value, None, "{mode}");
    }
}

#[test]
fn policy_violations_fault_without_reaching_the_device() {
    let (plugin, wire) = start("violate");
    let mut plugin = plugin.unwrap();
    let error = plugin.sample(Instant::now()).unwrap_err();
    assert!(
        error.message.contains("policy violation"),
        "{}",
        error.message
    );
    assert!(!wire.writes().iter().any(|w| w[0] == 0x7f));
    assert!(failsafe_written(&wire));

    let (plugin, wire) = start("io-in-describe");
    assert!(plugin.err().unwrap().contains("policy violation"));
    assert_eq!(wire.writes(), vec![FAILSAFE.to_vec()]);

    let (plugin, _) = start("wrong-describe");
    assert!(plugin.err().unwrap().contains("signed policy"));
}
