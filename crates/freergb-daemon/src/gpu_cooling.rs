//! GPU fan control with guaranteed return to the driver's automatic mode.
//!
//! Backends: NVIDIA through NVML (shipped with the driver, loaded at runtime
//! from the system directory; nothing is bundled), and a simulated backend for
//! tests, AMD on Windows through ADLX (`freergb-gpu-adlx`, same rules). AMD on
//! Linux is covered by the generic `amdgpu` hwmon headers in `hwmon.rs`; Intel
//! Arc control is not implemented (read-only temperatures via LHM/hwmon).
//!
//! Invariants, mirroring the motherboard providers:
//! * a fan id is journaled *before* the first write, so a crashed daemon hands
//!   every journaled fan back to driver auto on its next start;
//! * release, pause, daemon shutdown (`Drop`) and every write fault restore
//!   driver auto;
//! * duty never goes below the fan's floor (40% or the driver minimum);
//! * a missing/stale/fail-safe composite output means 100%.
use crate::composite_feed::CompositeFeed;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

const FAN_FLOOR: f32 = freergb_control::fan_support::SAFE_FAN_FLOOR as f32;
const MAX_ASSIGNMENTS: usize = 16;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GpuFan {
    /// Stable across restarts: `<backend>:<gpu uuid>/fan<N>`.
    pub id: String,
    pub vendor: &'static str,
    pub gpu_name: String,
    pub index: u32,
    pub duty_percent: Option<f32>,
    pub controllable: bool,
    /// Why control is unavailable, when it is.
    pub reason: Option<String>,
    pub min_percent: f32,
}

/// One vendor API. All calls are synchronous and bounded by the driver.
pub trait GpuBackend: Send {
    fn name(&self) -> &'static str;
    fn discover(&mut self) -> Vec<GpuFan>;
    fn take_control(&mut self, fan_id: &str) -> Result<(), String>;
    fn set_duty(&mut self, fan_id: &str, percent: f32) -> Result<(), String>;
    /// Hands the fan back to the driver's automatic policy.
    fn release(&mut self, fan_id: &str) -> Result<(), String>;
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum GpuMode {
    Manual {
        percent: f32,
    },
    /// Follows a composite curve output (`cooling.curve_set`).
    Composite {
        output: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GpuAssignment {
    pub fan_id: String,
    #[serde(flatten)]
    pub mode: GpuMode,
}

#[derive(Default, Deserialize, Serialize)]
struct Journal {
    version: u32,
    fans: Vec<String>,
}

struct Active {
    backend: usize,
    assignment: GpuAssignment,
    applied: Option<f32>,
}

#[derive(Default)]
pub struct GpuCooling {
    backends: Vec<Box<dyn GpuBackend>>,
    fans: Vec<(usize, GpuFan)>,
    active: BTreeMap<String, Active>,
    faults: BTreeMap<String, String>,
    state_dir: Option<PathBuf>,
    paused: bool,
    started: bool,
}

impl GpuCooling {
    pub fn new(backends: Vec<Box<dyn GpuBackend>>, state_dir: Option<PathBuf>) -> Self {
        let mut gpu = Self::default();
        gpu.backends = backends;
        gpu.state_dir = state_dir;
        gpu
    }

    /// Production backends available on this platform (none fail hard).
    pub fn platform_backends() -> Vec<Box<dyn GpuBackend>> {
        let mut out: Vec<Box<dyn GpuBackend>> = Vec::new();
        match nvml::Nvml::load() {
            Ok(nvml) => out.push(Box::new(nvml)),
            Err(error) => tracing::debug!(%error, "NVML not available; no NVIDIA fan control"),
        }
        match adlx::Adlx::load() {
            Ok(adlx) => out.push(Box::new(adlx)),
            Err(error) => {
                tracing::debug!(%error, "ADLX not available; no AMD fan control on Windows")
            }
        }
        out
    }

    fn journal_path(&self) -> Option<PathBuf> {
        self.state_dir
            .as_ref()
            .map(|d| d.join("gpu-control-journal.json"))
    }

    fn write_journal(&self) {
        let Some(path) = self.journal_path() else {
            return;
        };
        let journal = Journal {
            version: 1,
            fans: self.active.keys().cloned().collect(),
        };
        let tmp = path.with_extension("json.tmp");
        let result = serde_json::to_vec(&journal)
            .map_err(std::io::Error::other)
            .and_then(|bytes| std::fs::write(&tmp, bytes))
            .and_then(|()| std::fs::rename(&tmp, &path));
        if let Err(error) = result {
            tracing::warn!(%error, "GPU control journal not written");
        }
    }

    /// Releases fans a crashed daemon left in manual mode, then discovers.
    pub fn start(&mut self) {
        self.started = true;
        self.rescan();
        if let Some(path) = self.journal_path() {
            let journal = std::fs::read(&path)
                .ok()
                .filter(|b| b.len() <= 64 * 1024)
                .and_then(|b| serde_json::from_slice::<Journal>(&b).ok());
            for fan_id in journal.map(|j| j.fans).unwrap_or_default() {
                if let Some(backend) = self.backend_of(&fan_id) {
                    if let Err(error) = self.backends[backend].release(&fan_id) {
                        tracing::warn!(%fan_id, %error, "journaled GPU fan not released");
                    }
                }
            }
            let _ = std::fs::remove_file(path);
        }
    }

    pub fn rescan(&mut self) {
        self.fans = self
            .backends
            .iter_mut()
            .enumerate()
            .flat_map(|(i, b)| b.discover().into_iter().map(move |f| (i, f)))
            .collect();
    }

    fn backend_of(&self, fan_id: &str) -> Option<usize> {
        self.fans
            .iter()
            .find(|(_, f)| f.id == fan_id)
            .map(|(i, _)| *i)
    }

    fn floor(&self, fan_id: &str) -> f32 {
        self.fans
            .iter()
            .find(|(_, f)| f.id == fan_id)
            .map_or(100.0, |(_, f)| f.min_percent.max(FAN_FLOOR))
    }

    pub fn assign(&mut self, assignment: GpuAssignment) -> Result<(), String> {
        let (backend, fan) = self
            .fans
            .iter()
            .find(|(_, f)| f.id == assignment.fan_id)
            .cloned()
            .ok_or("unknown GPU fan; see cooling.gpu.list")?;
        if !fan.controllable {
            return Err(fan
                .reason
                .unwrap_or_else(|| "GPU fan is not controllable".into()));
        }
        let floor = self.floor(&fan.id);
        match &assignment.mode {
            GpuMode::Manual { percent }
                if !percent.is_finite() || *percent < floor || *percent > 100.0 =>
            {
                return Err(format!("percent must be {floor}..100"));
            }
            GpuMode::Composite { output } if output.is_empty() || output.len() > 64 => {
                return Err("composite output name required".into());
            }
            _ => {}
        }
        if !self.active.contains_key(&fan.id) {
            if self.active.len() >= MAX_ASSIGNMENTS {
                return Err("too many GPU fan assignments".into());
            }
            // Journal first: a crash between take_control and the next tick must
            // still be recoverable.
            self.active.insert(
                fan.id.clone(),
                Active {
                    backend,
                    assignment: assignment.clone(),
                    applied: None,
                },
            );
            self.write_journal();
            if !self.paused {
                if let Err(error) = self.backends[backend].take_control(&fan.id) {
                    let _ = self.backends[backend].release(&fan.id);
                    self.active.remove(&fan.id);
                    self.write_journal();
                    return Err(error);
                }
            }
        }
        self.faults.remove(&fan.id);
        if let Some(active) = self.active.get_mut(&fan.id) {
            active.assignment = assignment;
        }
        Ok(())
    }

    /// Returns one fan (or all with `None`) to driver auto.
    pub fn release(&mut self, fan_id: Option<&str>) -> Result<usize, String> {
        let ids: Vec<String> = match fan_id {
            Some(id) => self.active.keys().filter(|k| *k == id).cloned().collect(),
            None => self.active.keys().cloned().collect(),
        };
        let mut errors = Vec::new();
        for id in &ids {
            if let Some(active) = self.active.remove(id) {
                if let Err(e) = self.backends[active.backend].release(id) {
                    errors.push(format!("{id}: {e}"));
                }
            }
        }
        self.write_journal();
        if errors.is_empty() {
            Ok(ids.len())
        } else {
            Err(errors.join("; "))
        }
    }

    /// Pause hands every fan to the driver without forgetting assignments.
    pub fn set_paused(&mut self, paused: bool) -> Result<bool, String> {
        if paused == self.paused {
            return Ok(paused);
        }
        self.paused = paused;
        let mut errors = Vec::new();
        for (id, active) in &mut self.active {
            let backend = &mut self.backends[active.backend];
            let result = if paused {
                backend.release(id)
            } else {
                backend.take_control(id)
            };
            active.applied = None;
            if let Err(e) = result {
                errors.push(format!("{id}: {e}"));
            }
        }
        if errors.is_empty() {
            Ok(paused)
        } else {
            Err(errors.join("; "))
        }
    }

    /// One control step (called every second). A failed write releases that
    /// fan to driver auto and records a fault.
    pub fn tick(&mut self, feed: &CompositeFeed, now: Instant) {
        if self.paused {
            return;
        }
        let ids: Vec<String> = self.active.keys().cloned().collect();
        for id in ids {
            let floor = self.floor(&id);
            let active = self.active.get_mut(&id).expect("listed above");
            let target = match &active.assignment.mode {
                GpuMode::Manual { percent } => *percent,
                GpuMode::Composite { output } => feed.duty(output, now),
            }
            .clamp(floor, 100.0);
            let backend = &mut self.backends[active.backend];
            match backend.set_duty(&id, target) {
                Ok(()) => active.applied = Some(target),
                Err(error) => {
                    let released = backend.release(&id);
                    self.active.remove(&id);
                    let message = match released {
                        Ok(()) => format!("write failed, returned to driver auto: {error}"),
                        Err(e) => format!("write failed ({error}); release failed: {e}"),
                    };
                    tracing::warn!(fan = %id, %message, "GPU fan fault");
                    self.faults.insert(id, message);
                    self.write_journal();
                }
            }
        }
    }

    pub fn view(&self) -> Value {
        let fans: Vec<Value> = self
            .fans
            .iter()
            .map(|(_, f)| {
                let active = self.active.get(&f.id);
                json!({
                    "fan": f,
                    "floor_percent": self.floor(&f.id),
                    "assignment": active.map(|a| &a.assignment),
                    "applied_percent": active.and_then(|a| a.applied),
                    "fault": self.faults.get(&f.id),
                })
            })
            .collect();
        json!({
            "started": self.started,
            "paused": self.paused,
            "backends": self.backends.iter().map(|b| b.name()).collect::<Vec<_>>(),
            "fans": fans,
        })
    }
}

impl Drop for GpuCooling {
    fn drop(&mut self) {
        if let Err(error) = self.release(None) {
            tracing::warn!(%error, "GPU fans not released on shutdown");
        }
    }
}

/// Deterministic in-memory backend for tests and `--simulate`-style demos.
pub mod simulated {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct SimFan {
        pub manual: bool,
        pub duty: f32,
        pub fail_writes: bool,
        pub writes: u32,
    }

    /// Shared state so a test can inspect fans after the controller is dropped.
    #[derive(Clone, Default)]
    pub struct SimulatedGpu(pub Arc<Mutex<BTreeMap<String, SimFan>>>);

    impl SimulatedGpu {
        pub fn with_fans(ids: &[&str]) -> Self {
            let sim = Self::default();
            for id in ids {
                sim.0.lock().unwrap().insert(
                    (*id).into(),
                    SimFan {
                        duty: 30.0,
                        ..SimFan::default()
                    },
                );
            }
            sim
        }
        pub fn fan(&self, id: &str) -> SimFan {
            self.0.lock().unwrap()[id].clone()
        }
        fn with<T>(
            &self,
            id: &str,
            f: impl FnOnce(&mut SimFan) -> Result<T, String>,
        ) -> Result<T, String> {
            let mut fans = self.0.lock().unwrap();
            f(fans.get_mut(id).ok_or("no such simulated fan")?)
        }
    }

    impl GpuBackend for SimulatedGpu {
        fn name(&self) -> &'static str {
            "simulated"
        }
        fn discover(&mut self) -> Vec<GpuFan> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, (id, f))| GpuFan {
                    id: id.clone(),
                    vendor: "simulated",
                    gpu_name: "Simulated GPU".into(),
                    index: i as u32,
                    duty_percent: Some(f.duty),
                    controllable: true,
                    reason: None,
                    min_percent: 30.0,
                })
                .collect()
        }
        fn take_control(&mut self, id: &str) -> Result<(), String> {
            self.with(id, |f| {
                f.manual = true;
                Ok(())
            })
        }
        fn set_duty(&mut self, id: &str, percent: f32) -> Result<(), String> {
            self.with(id, |f| {
                if f.fail_writes || !f.manual {
                    return Err("simulated write failure".into());
                }
                f.duty = percent;
                f.writes += 1;
                Ok(())
            })
        }
        fn release(&mut self, id: &str) -> Result<(), String> {
            self.with(id, |f| {
                f.manual = false;
                f.duty = 30.0;
                Ok(())
            })
        }
    }
}

/// NVIDIA Management Library adapter. The FFI lives in `freergb-gpu-nvml` so
/// this crate keeps `unsafe_code = "forbid"`.
mod nvml {
    use super::{GpuBackend, GpuFan};

    pub struct Nvml(freergb_gpu_nvml::Nvml);

    impl Nvml {
        pub fn load() -> Result<Self, String> {
            freergb_gpu_nvml::Nvml::load().map(Self)
        }
    }

    impl GpuBackend for Nvml {
        fn name(&self) -> &'static str {
            "nvml"
        }
        fn discover(&mut self) -> Vec<GpuFan> {
            self.0
                .discover()
                .into_iter()
                .map(|f| GpuFan {
                    id: f.id,
                    vendor: "nvidia",
                    gpu_name: f.gpu_name,
                    index: f.index,
                    duty_percent: f.duty_percent,
                    controllable: f.controllable,
                    reason: f.reason,
                    min_percent: f.min_percent,
                })
                .collect()
        }
        fn take_control(&mut self, id: &str) -> Result<(), String> {
            self.0.take_control(id)
        }
        fn set_duty(&mut self, id: &str, percent: f32) -> Result<(), String> {
            self.0.set_duty(id, percent)
        }
        fn release(&mut self, id: &str) -> Result<(), String> {
            self.0.release(id)
        }
    }
}

/// AMD ADLX adapter (Windows). The FFI lives in `freergb-gpu-adlx`. ADLX is
/// the control path for AMD GPUs; LHM `/gpu-amd/*/control` stays read-only
/// (docs/COOLING_SOURCES_GPU.md).
mod adlx {
    use super::{GpuBackend, GpuFan};

    pub struct Adlx(freergb_gpu_adlx::Adlx);

    impl Adlx {
        pub fn load() -> Result<Self, String> {
            freergb_gpu_adlx::Adlx::load().map(Self)
        }
    }

    impl GpuBackend for Adlx {
        fn name(&self) -> &'static str {
            "adlx"
        }
        fn discover(&mut self) -> Vec<GpuFan> {
            self.0
                .discover()
                .into_iter()
                .map(|f| GpuFan {
                    id: f.id,
                    vendor: "amd",
                    gpu_name: f.gpu_name,
                    index: f.index,
                    duty_percent: f.duty_percent,
                    controllable: f.controllable,
                    reason: f.reason,
                    min_percent: f.min_percent,
                })
                .collect()
        }
        fn take_control(&mut self, id: &str) -> Result<(), String> {
            self.0.take_control(id)
        }
        fn set_duty(&mut self, id: &str, percent: f32) -> Result<(), String> {
            self.0.set_duty(id, percent)
        }
        fn release(&mut self, id: &str) -> Result<(), String> {
            self.0.release(id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::simulated::SimulatedGpu;
    use super::*;
    use freergb_control::curves::CurveOutput;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("freergb-gpu-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn controller(sim: &SimulatedGpu, state: &std::path::Path) -> GpuCooling {
        let mut gpu = GpuCooling::new(vec![Box::new(sim.clone())], Some(state.into()));
        gpu.start();
        gpu
    }

    fn manual(percent: f32) -> GpuAssignment {
        GpuAssignment {
            fan_id: "sim:gpu0/fan0".into(),
            mode: GpuMode::Manual { percent },
        }
    }

    #[test]
    fn take_set_and_release_return_to_driver_auto() {
        let sim = SimulatedGpu::with_fans(&["sim:gpu0/fan0"]);
        let state = dir("release");
        let mut gpu = controller(&sim, &state);
        assert!(gpu.assign(manual(20.0)).is_err(), "below floor is rejected");
        gpu.assign(manual(65.0)).unwrap();
        assert!(sim.fan("sim:gpu0/fan0").manual);
        gpu.tick(&CompositeFeed::default(), Instant::now());
        assert_eq!(sim.fan("sim:gpu0/fan0").duty, 65.0);
        assert!(state.join("gpu-control-journal.json").exists());
        assert_eq!(gpu.release(None).unwrap(), 1);
        assert!(!sim.fan("sim:gpu0/fan0").manual);
    }

    #[test]
    fn composite_missing_or_failsafe_means_full_duty() {
        let sim = SimulatedGpu::with_fans(&["sim:gpu0/fan0"]);
        let mut gpu = controller(&sim, &dir("composite"));
        gpu.assign(GpuAssignment {
            fan_id: "sim:gpu0/fan0".into(),
            mode: GpuMode::Composite {
                output: "gpu".into(),
            },
        })
        .unwrap();
        let feed = CompositeFeed::default();
        let now = Instant::now();
        gpu.tick(&feed, now);
        assert_eq!(sim.fan("sim:gpu0/fan0").duty, 100.0);
        feed.publish(
            [(
                "gpu".into(),
                CurveOutput {
                    percent: 10.0,
                    failsafe: false,
                },
            )]
            .into(),
            now,
        );
        gpu.tick(&feed, now);
        assert_eq!(sim.fan("sim:gpu0/fan0").duty, 40.0, "floor applies");
        feed.publish(
            [(
                "gpu".into(),
                CurveOutput {
                    percent: 55.0,
                    failsafe: false,
                },
            )]
            .into(),
            now,
        );
        gpu.tick(&feed, now);
        assert_eq!(sim.fan("sim:gpu0/fan0").duty, 55.0);
        feed.publish(
            [(
                "gpu".into(),
                CurveOutput {
                    percent: 55.0,
                    failsafe: true,
                },
            )]
            .into(),
            now,
        );
        gpu.tick(&feed, now);
        assert_eq!(sim.fan("sim:gpu0/fan0").duty, 100.0);
    }

    #[test]
    fn write_fault_releases_and_records() {
        let sim = SimulatedGpu::with_fans(&["sim:gpu0/fan0"]);
        let mut gpu = controller(&sim, &dir("fault"));
        gpu.assign(manual(50.0)).unwrap();
        sim.0
            .lock()
            .unwrap()
            .get_mut("sim:gpu0/fan0")
            .unwrap()
            .fail_writes = true;
        gpu.tick(&CompositeFeed::default(), Instant::now());
        assert!(!sim.fan("sim:gpu0/fan0").manual);
        let view = gpu.view();
        assert!(view["fans"][0]["fault"]
            .as_str()
            .unwrap()
            .contains("driver auto"));
        assert!(view["fans"][0]["assignment"].is_null());
    }

    #[test]
    fn shutdown_and_pause_release_and_crash_journal_recovers() {
        let sim = SimulatedGpu::with_fans(&["sim:gpu0/fan0", "sim:gpu0/fan1"]);
        let state = dir("crash");
        let mut gpu = controller(&sim, &state);
        gpu.assign(manual(70.0)).unwrap();
        gpu.set_paused(true).unwrap();
        assert!(!sim.fan("sim:gpu0/fan0").manual);
        gpu.set_paused(false).unwrap();
        assert!(sim.fan("sim:gpu0/fan0").manual);
        drop(gpu);
        assert!(!sim.fan("sim:gpu0/fan0").manual, "drop releases");

        // Crash: controller leaks without Drop, fan stays manual.
        let mut gpu = controller(&sim, &state);
        gpu.assign(manual(70.0)).unwrap();
        std::mem::forget(gpu);
        assert!(sim.fan("sim:gpu0/fan0").manual);
        let gpu = controller(&sim, &state);
        assert!(
            !sim.fan("sim:gpu0/fan0").manual,
            "next start releases journaled fan"
        );
        assert!(!state.join("gpu-control-journal.json").exists());
        drop(gpu);
    }

    #[test]
    fn unknown_and_read_only_fans_are_refused() {
        let sim = SimulatedGpu::with_fans(&["sim:gpu0/fan0"]);
        let mut gpu = controller(&sim, &dir("unknown"));
        let mut a = manual(50.0);
        a.fan_id = "nope".into();
        assert!(gpu.assign(a).is_err());
        gpu.fans[0].1.controllable = false;
        gpu.fans[0].1.reason = Some("needs admin".into());
        assert_eq!(gpu.assign(manual(50.0)).unwrap_err(), "needs admin");
    }
}
