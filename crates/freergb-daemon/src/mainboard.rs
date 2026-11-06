//! Optional, read-only motherboard provider. Blocking native access stays in a child.
use freergb_device_api::SensorReading;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const MAX_MESSAGE: usize = 256 * 1024;
const STALE_MS: u64 = 5000;
/// Pause before a new helper session after the provider was unavailable or failed.
const RETRY: Duration = Duration::from_secs(5);

/// Protected inventory of what a control can reach (no I/O performed for it).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeSupport {
    /// "verified" (audited register readback) or "lhm_supported".
    pub tier: String,
    pub register_readback: bool,
    /// LHM already reports software mode: another tool owns it; never taken over.
    pub software_mode: bool,
    /// A crashed lower-tier worker's channel is held at 100% until reboot.
    pub held_full: bool,
}

/// Public SMBIOS identity; unrestricted unlocks are bound to it.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoardIdentity {
    pub manufacturer: String,
    pub product: String,
    pub bios_version: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeSensor {
    id: String,
    name: String,
    kind: String,
    value: Option<f64>,
    age_ms: u64,
    control_available: bool,
    #[serde(default)]
    support: Option<NativeSupport>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hardware {
    id: String,
    name: String,
    sensors: Vec<NativeSensor>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u32,
    status: String,
    message: String,
    hardware: Vec<Hardware>,
    #[serde(default)]
    board: Option<BoardIdentity>,
}

impl Snapshot {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_MESSAGE {
            return Err("mainboard reply too large".into());
        }
        let snapshot: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if snapshot.version != 1
            || ![
                "ready",
                "missing_library",
                "missing_driver",
                "permission_denied",
                "no_supported_hardware",
                "faulted",
            ]
            .contains(&snapshot.status.as_str())
        {
            return Err("unsupported mainboard reply version/status".into());
        }
        let valid_text =
            |s: &str| !s.is_empty() && s.len() <= 512 && !s.chars().any(char::is_control);
        if snapshot.message.len() > 2048 || snapshot.hardware.len() > 64 {
            return Err("mainboard inventory exceeds bounds".into());
        }
        if snapshot.status != "ready" && !snapshot.hardware.is_empty() {
            return Err("unavailable provider returned hardware".into());
        }
        if let Some(board) = &snapshot.board {
            let bounded = |s: &str| s.len() <= 128 && !s.chars().any(char::is_control);
            if !bounded(&board.manufacturer)
                || !bounded(&board.product)
                || !bounded(&board.bios_version)
            {
                return Err("invalid motherboard identity".into());
            }
        }
        let mut ids = BTreeSet::new();
        let mut count = 0;
        for hardware in &snapshot.hardware {
            if !valid_text(&hardware.id)
                || !valid_text(&hardware.name)
                || !ids.insert(hardware.id.clone())
            {
                return Err("invalid/duplicate motherboard identity".into());
            }
            let mut sensors = BTreeSet::new();
            for sensor in &hardware.sensors {
                count += 1;
                if count > 1024
                    || !valid_text(&sensor.id)
                    || !valid_text(&sensor.name)
                    || !sensors.insert(&sensor.id)
                {
                    return Err("invalid/duplicate motherboard sensor".into());
                }
                let range = match sensor.kind.as_str() {
                    "temperature" => -100.0..=200.0,
                    "fan" => 0.0..=100_000.0,
                    "control" => 0.0..=100.0,
                    _ => return Err("unknown motherboard sensor kind".into()),
                };
                if sensor
                    .value
                    .is_some_and(|v| !v.is_finite() || !range.contains(&v))
                {
                    return Err("invalid motherboard sensor value".into());
                }
                if sensor.control_available && sensor.kind != "control" {
                    return Err("control capability on non-control sensor".into());
                }
                if let Some(support) = &sensor.support {
                    if !sensor.control_available
                        || !["verified", "lhm_supported"].contains(&support.tier.as_str())
                        || support.register_readback != (support.tier == "verified")
                    {
                        return Err("invalid motherboard control support".into());
                    }
                }
            }
        }
        Ok(snapshot)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ProviderSummary {
    pub id: &'static str,
    pub name: &'static str,
    pub status: String,
    pub message: String,
    pub read_only: bool,
    pub control_enabled: bool,
    pub rgb_supported: bool,
    pub age_ms: Option<u64>,
    pub board: Option<BoardIdentity>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SensorEntry {
    pub provider: &'static str,
    pub device_id: String,
    pub device_name: String,
    pub sensor: SensorReading,
}

#[derive(Clone, Debug, Serialize)]
pub struct Channel {
    pub provider: &'static str,
    pub id: String,
    pub device_id: String,
    pub name: String,
    pub kind: String,
    pub reading: SensorReading,
    pub control_available: bool,
    pub writable: bool,
    pub physical_mapping: &'static str,
    pub electrical_mode: &'static str,
    /// Raw native sensor identity (as used by sessions).
    pub sensor_id: String,
    pub hardware_name: String,
    pub support: Option<NativeSupport>,
}

struct Cache {
    status: String,
    message: String,
    snapshot: Option<(Snapshot, Instant)>,
}

pub struct MainboardProvider {
    cache: Arc<Mutex<Cache>>,
    stop: Arc<(Mutex<bool>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}

impl Default for MainboardProvider {
    fn default() -> Self {
        Self {
            cache: Arc::new(Mutex::new(Cache {
                status: "disabled".into(),
                message: "Optional motherboard provider is not enabled.".into(),
                snapshot: None,
            })),
            stop: Arc::new((Mutex::new(false), Condvar::new())),
            worker: None,
        }
    }
}

impl MainboardProvider {
    pub fn start(
        &mut self,
        helper: PathBuf,
        library: PathBuf,
        hardware_allowed: bool,
    ) -> Result<(), String> {
        if self.worker.is_some() {
            return Err("motherboard provider already started".into());
        }
        if !hardware_allowed {
            return Err("--no-hardware blocks the motherboard provider".into());
        }
        if !cfg!(windows) {
            return Err("motherboard provider currently requires Windows".into());
        }
        let helper = helper
            .canonicalize()
            .map_err(|_| "motherboard helper is missing; build/install freergb-mainboard.exe")?;
        let library = library
            .canonicalize()
            .map_err(|_| "motherboard library directory does not exist")?;
        if !helper.is_file() || !library.is_dir() {
            return Err("invalid motherboard helper/library path".into());
        }
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.status = "starting".into();
            cache.message = "Checking the optional motherboard provider prerequisites.".into();
        }
        let cache = self.cache.clone();
        let stop = self.stop.clone();
        self.worker = Some(
            std::thread::Builder::new()
                .name("mainboard-sensors".into())
                .spawn(move || loop {
                    let result = Session::run(helper.clone(), library.clone(), &cache, &stop);
                    if let Err(error) = result {
                        let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
                        cache.status = "faulted".into();
                        cache.message = error;
                        cache.snapshot = None;
                    }
                    // The protected broker may start after the daemon (installer,
                    // boot): retry unavailable or failed sessions until stopped.
                    let guard = stop.0.lock().unwrap_or_else(|e| e.into_inner());
                    let (guard, _) = stop
                        .1
                        .wait_timeout_while(guard, RETRY, |stop| !*stop)
                        .unwrap_or_else(|e| e.into_inner());
                    if *guard {
                        return;
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    pub fn summary(&self) -> ProviderSummary {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let age_ms = cache.snapshot.as_ref().map(|(_, at)| elapsed_ms(*at));
        let stale = cache.status == "ready" && age_ms.is_none_or(|age| age > STALE_MS);
        let board = cache.snapshot.as_ref().and_then(|(s, _)| s.board.clone());
        ProviderSummary {
            board,
            id: "mainboard.lhm",
            name: "LibreHardwareMonitor motherboard",
            status: if stale {
                "stale".into()
            } else {
                cache.status.clone()
            },
            message: if stale {
                "Motherboard provider readings expired.".into()
            } else {
                cache.message.clone()
            },
            read_only: true,
            control_enabled: false,
            rgb_supported: false,
            age_ms,
        }
    }

    pub fn sensors(&self) -> Vec<SensorEntry> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let Some((snapshot, at)) = &cache.snapshot else {
            return Vec::new();
        };
        let elapsed = elapsed_ms(*at);
        snapshot
            .hardware
            .iter()
            .flat_map(|hardware| {
                hardware.sensors.iter().map(move |sensor| SensorEntry {
                    provider: "mainboard.lhm",
                    device_id: format!("mainboard.lhm:{}", hardware.id),
                    device_name: hardware.name.clone(),
                    sensor: reading(sensor, elapsed),
                })
            })
            .collect()
    }

    pub fn channels(&self) -> Vec<Channel> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let Some((snapshot, at)) = &cache.snapshot else {
            return Vec::new();
        };
        let elapsed = elapsed_ms(*at);
        snapshot
            .hardware
            .iter()
            .flat_map(|hardware| {
                hardware
                    .sensors
                    .iter()
                    .filter(|s| s.kind == "fan" || s.kind == "control")
                    .map(move |sensor| Channel {
                        provider: "mainboard.lhm",
                        id: serde_json::to_string(&(&hardware.id, &sensor.id)).unwrap_or_default(),
                        device_id: format!("mainboard.lhm:{}", hardware.id),
                        name: sensor.name.clone(),
                        kind: sensor.kind.clone(),
                        reading: reading(sensor, elapsed),
                        control_available: sensor.control_available,
                        writable: false,
                        physical_mapping: "unconfirmed",
                        electrical_mode: "unknown",
                        sensor_id: sensor.id.clone(),
                        hardware_name: hardware.name.clone(),
                        support: sensor.support.clone(),
                    })
            })
            .collect()
    }
}

fn elapsed_ms(at: Instant) -> u64 {
    at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}
fn reading(sensor: &NativeSensor, elapsed: u64) -> SensorReading {
    let age_ms = sensor.age_ms.saturating_add(elapsed);
    SensorReading {
        id: sensor.id.clone(),
        name: sensor.name.clone(),
        unit: match sensor.kind.as_str() {
            "temperature" => "°C",
            "fan" => "RPM",
            _ => "%",
        }
        .into(),
        value: if age_ms > STALE_MS {
            None
        } else {
            sensor.value
        },
        age_ms,
    }
}

struct Session {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Session {
    fn run(
        helper: PathBuf,
        library: PathBuf,
        cache: &Mutex<Cache>,
        stop: &(Mutex<bool>, Condvar),
    ) -> Result<(), String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        runtime.block_on(async {
            let mut command = Command::new(helper);
            command
                .arg("--library")
                .arg(library)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true);
            #[cfg(windows)]
            command.creation_flags(0x08000000);
            let mut child = command.spawn().map_err(|e| e.to_string())?;
            let input = child.stdin.take().ok_or("missing helper stdin")?;
            let output = BufReader::new(child.stdout.take().ok_or("missing helper stdout")?);
            let mut session = Self {
                child,
                input,
                output,
            };
            let result = session.poll(cache, stop).await;
            // EOF exits the dedicated reader; stock LHM Close would request fan
            // control resets, so the helper leaves handle cleanup to process exit.
            let _ = session.input.shutdown().await;
            drop(session.input);
            if tokio::time::timeout(Duration::from_secs(1), session.child.wait())
                .await
                .is_err()
            {
                let _ = session.child.kill().await;
            }
            result
        })
    }

    async fn poll(
        &mut self,
        cache: &Mutex<Cache>,
        stop: &(Mutex<bool>, Condvar),
    ) -> Result<(), String> {
        let mut initial = true;
        loop {
            if *stop.0.lock().unwrap_or_else(|e| e.into_inner()) {
                return Ok(());
            }
            // Steady deadline matches the broker worker deadline (6 s, acceptance item 8).
            let timeout = Duration::from_secs(if initial { 10 } else { 6 });
            let request_at = Instant::now();
            let request = tokio::time::timeout(timeout, async {
                self.input
                    .write_all(b"snapshot\n")
                    .await
                    .map_err(|e| e.to_string())?;
                self.input.flush().await.map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                (&mut self.output)
                    .take((MAX_MESSAGE + 1) as u64)
                    .read_until(b'\n', &mut bytes)
                    .await
                    .map_err(|e| e.to_string())?;
                if bytes.len() > MAX_MESSAGE || bytes.last() != Some(&b'\n') {
                    return Err("oversized/incomplete motherboard reply".into());
                }
                Snapshot::parse(&bytes)
            });
            let reply = tokio::select! {
                result = request => result.map_err(|_| "motherboard helper timed out".to_string())??,
                _ = async {
                    loop {
                        if *stop.0.lock().unwrap_or_else(|e|e.into_inner()) { break; }
                        tokio::time::sleep(Duration::from_millis(25)).await;
                    }
                } => return Ok(()),
            };
            initial = false;
            let ready = reply.status == "ready";
            {
                let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
                cache.status = reply.status.clone();
                cache.message = reply.message.clone();
                // Anchor before the request: native latency must not manufacture freshness.
                cache.snapshot = ready.then_some((reply, request_at));
            }
            if !ready {
                return Ok(());
            }
            let guard = stop.0.lock().unwrap_or_else(|e| e.into_inner());
            let _ = stop
                .1
                .wait_timeout_while(guard, Duration::from_secs(1), |stop| !*stop)
                .unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl Drop for MainboardProvider {
    fn drop(&mut self) {
        *self.stop.0.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.stop.1.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> serde_json::Value {
        json!({"version":1,"status":"ready","message":"fixture only","hardware":[{"id":"/board/it8689e","name":"Fixture board","sensors":[{"id":"/fan/0","name":"Fan #1","kind":"fan","value":900.0,"age_ms":0,"control_available":false},{"id":"/control/0","name":"Fan control #1","kind":"control","value":40.0,"age_ms":0,"control_available":true},{"id":"/temperature/0","name":"Temperature #1","kind":"temperature","value":35.0,"age_ms":0,"control_available":false}]}]})
    }

    #[test]
    fn rejects_wrong_versions_duplicate_ids_and_invalid_values() {
        let value = fixture();
        assert!(Snapshot::parse(&serde_json::to_vec(&value).unwrap()).is_ok());
        for (field, replacement) in [("version", json!(2)), ("status", json!("missing_driver"))] {
            let mut bad = value.clone();
            bad[field] = replacement;
            assert!(Snapshot::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        for (field, replacement) in [
            ("value", json!(-1)),
            ("kind", json!("rgb")),
            ("control_available", json!(true)),
        ] {
            let mut bad = value.clone();
            bad["hardware"][0]["sensors"][0][field] = replacement;
            assert!(Snapshot::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = value.clone();
        let duplicate = bad["hardware"][0]["sensors"][0].clone();
        bad["hardware"][0]["sensors"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(Snapshot::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        assert!(Snapshot::parse(&vec![b' '; MAX_MESSAGE + 1]).is_err());
    }

    #[test]
    fn channels_are_read_only_unpaired_and_expire_without_gui() {
        let provider = MainboardProvider::default();
        let snapshot = Snapshot::parse(&serde_json::to_vec(&fixture()).unwrap()).unwrap();
        {
            let mut cache = provider.cache.lock().unwrap();
            cache.status = "ready".into();
            cache.snapshot = Some((snapshot, Instant::now()));
        }
        assert_eq!(provider.sensors().len(), 3);
        let channels = provider.channels();
        assert_eq!(channels.len(), 2);
        assert!(channels.iter().all(|c| !c.writable
            && c.physical_mapping == "unconfirmed"
            && c.electrical_mode == "unknown"));
        assert!(channels[1].control_available);
        provider.cache.lock().unwrap().snapshot.as_mut().unwrap().1 =
            Instant::now() - Duration::from_secs(6);
        assert_eq!(provider.summary().status, "stale");
        assert!(provider.sensors().iter().all(|s| s.sensor.value.is_none()));
        assert!(provider
            .channels()
            .iter()
            .all(|c| c.reading.value.is_none()));
    }

    #[test]
    fn no_hardware_blocks_process_creation() {
        let mut provider = MainboardProvider::default();
        assert!(provider
            .start("unused.exe".into(), "unused".into(), false)
            .unwrap_err()
            .contains("--no-hardware"));
        assert_eq!(provider.summary().status, "disabled");
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "run scripts/test-mainboard-provider.ps1 to build no-hardware fixtures"]
    fn helper_transport_isolated_missing_runtime_faults_and_deadline() {
        let real_helper = PathBuf::from(
            std::env::var_os("FREERGB_TEST_MAINBOARD_HELPER").expect("test script sets helper"),
        );
        let fixture = PathBuf::from(
            std::env::var_os("FREERGB_TEST_MAINBOARD_FIXTURE").expect("test script sets fixture"),
        );
        let directory =
            std::env::temp_dir().join(format!("freergb provider test {}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        for scenario in [
            "missing",
            "ready",
            "bad",
            "exit",
            "oversized",
            "hang",
            "cancel",
            "denied_then_ready",
        ] {
            let _ = std::fs::remove_file(directory.join("denied.marker"));
            std::fs::write(directory.join("scenario.txt"), scenario).unwrap();
            let mut provider = MainboardProvider::default();
            provider
                .start(
                    if scenario == "missing" {
                        real_helper.clone()
                    } else {
                        fixture.clone()
                    },
                    directory.clone(),
                    true,
                )
                .unwrap();
            if scenario == "cancel" {
                std::thread::sleep(Duration::from_millis(150));
                let shutdown = Instant::now();
                drop(provider);
                assert!(
                    shutdown.elapsed() < Duration::from_secs(2),
                    "active native request must be cancellable"
                );
                continue;
            }
            if scenario == "denied_then_ready" {
                // The first session reports permission_denied; the 5 s retry must reach ready.
                let deadline = Instant::now() + Duration::from_secs(15);
                let mut saw_denied = false;
                while provider.summary().status != "ready" && Instant::now() < deadline {
                    saw_denied |= provider.summary().status == "permission_denied";
                    std::thread::sleep(Duration::from_millis(20));
                }
                assert!(
                    saw_denied,
                    "first helper session must report permission_denied"
                );
                assert_eq!(provider.summary().status, "ready", "retry must recover");
                assert_eq!(provider.sensors()[0].sensor.value, Some(900.0));
                drop(provider);
                let _ = std::fs::remove_file(directory.join("denied.marker"));
                continue;
            }
            let deadline = Instant::now() + Duration::from_secs(12);
            while provider.summary().status == "starting" && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            let expected = if scenario == "missing" {
                "missing_library"
            } else if scenario == "ready" {
                "ready"
            } else {
                "faulted"
            };
            assert_eq!(
                provider.summary().status,
                expected,
                "{scenario}: {}",
                provider.summary().message
            );
            if scenario == "ready" {
                assert_eq!(provider.sensors()[0].sensor.value, Some(900.0));
            } else {
                assert!(provider.sensors().is_empty() && provider.channels().is_empty());
            }
            let shutdown = Instant::now();
            drop(provider);
            assert!(
                shutdown.elapsed() < Duration::from_secs(2),
                "fixture must be reaped: {scenario}"
            );
        }
        std::fs::remove_file(directory.join("scenario.txt")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
