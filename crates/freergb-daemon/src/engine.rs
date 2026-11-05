//! Render engine: one effect draws onto a shared canvas, every device samples
//! its region, applies its colour correction and receives the frame.
//!
//! Each device runs on its own worker thread that owns the device. `tick`
//! only renders and drops the newest frame into the worker's slot; a slow
//! device skips stale frames instead of slowing the others down. Workers also
//! handle connecting and retrying with a fixed back-off.

use crate::state;
use freergb_core::{Canvas, ColorCorrection, Frame, LedLayout, Region};
use freergb_device_api::{DeviceId, DeviceInfo, DeviceStatus, RgbDevice};
use freergb_effects::{Effect, EffectSpec};
use freergb_plugins::{Access, Manifest, ProcessDevice};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const RETRY_INTERVAL: Duration = Duration::from_secs(2);
/// One slow rise and fall: far below any flash/strobe rate.
pub const LIGHT_TEST: Duration = Duration::from_millis(2400);
/// Upper bound for the exponential reconnect back-off.
pub const MAX_RETRY: Duration = Duration::from_secs(60);
const CANVAS_W: usize = 320;
const CANVAS_H: usize = 200;
/// How long an idle worker sleeps before re-checking retries / stop.
const WORKER_POLL: Duration = Duration::from_millis(100);

fn guard<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// What the engine and the API may see of a device without touching it.
#[derive(Debug, Clone)]
struct View {
    key_map: std::collections::BTreeMap<u16, usize>,
    sensors: Vec<freergb_device_api::SensorReading>,
    sensor_snapshot_at: Instant,
    info: DeviceInfo,
    status: DeviceStatus,
    layout: LedLayout,
    frames: u64,
    dropped: u64,
    last_error: Option<String>,
}

/// Newest pending frame for a worker (`None` = nothing new).
type Slot = Arc<(Mutex<Option<Frame>>, Condvar)>;

struct Worker {
    id: DeviceId,
    view: Arc<Mutex<View>>,
    slot: Slot,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    region: Region,
    correction: ColorCorrection,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceSummary {
    pub sensors: Vec<freergb_device_api::SensorReading>,
    pub info: DeviceInfo,
    pub status: DeviceStatus,
    /// Per-device brightness (multiplied with the global brightness).
    pub brightness: f32,
    /// Where the device sits on the effect canvas (normalised).
    pub region: Region,
    pub leds: usize,
    pub frames: u64,
    /// Frames replaced before the device could take them (device slower than the engine).
    pub dropped: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    /// Audio capture device while an audio effect runs, or the error.
    pub audio: Option<String>,
    /// Captured monitor while a screen effect runs, or the error.
    pub screen: Option<String>,
    /// Other RGB software currently running; hardware is left alone meanwhile.
    pub conflicts: Vec<&'static str>,
    pub uptime_s: f64,
    pub ticks: u64,
    pub target_fps: u32,
    pub measured_fps: f64,
    pub effect: EffectSpec,
    pub brightness: f32,
    pub devices: usize,
    /// Support tier per motherboard fan control (no readings or identities).
    pub cooling_support: Vec<CoolingSupportEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoolingSupportEntry {
    pub name: String,
    pub tier: String,
    pub register_readback: bool,
    pub software_mode: bool,
    pub held_full: bool,
}

/// Pseudo-conflict while the user paused lighting (shown like other conflicts).
pub const PAUSED_BY_USER: &str = "Paused by you";

pub struct Engine {
    pub marketplace: Option<Arc<freergb_effect_host::marketplace::Manager>>,
    effect_fault: freergb_effect_host::runtime::Fault,
    activation_error: Option<String>,
    pub cooling_sessions: crate::cooling_sessions::CoolingSessions,
    pub cooling_assignments: crate::cooling_assignments::Assignments,
    pub background: crate::background::Background,
    pub fan_support: crate::fan_support::FanSupport,
    pub cooling_profiles: crate::cooling_profiles::CoolingProfiles,
    pub onboarding: crate::onboarding::Onboarding,
    pub mainboard_rgb: crate::mainboard_rgb::MainboardRgb,
    recorded_test_job: Option<u64>,
    pub channel_bindings: crate::channel_bindings::Bindings,
    pub keyboard: crate::keyboard::KeyboardReactive,
    pub controls: crate::control::Controls,
    pub mainboard: crate::mainboard::MainboardProvider,
    /// Linux kernel hwmon fan headers (not started on other platforms).
    pub hwmon: crate::hwmon::HwmonProvider,
    /// GPU fans (NVML); started only with hardware access.
    pub gpu: crate::gpu_cooling::GpuCooling,
    sensor_refresh_at: Option<Instant>,
    plugins: std::collections::BTreeMap<String, PluginEntry>,
    /// Hardware module setup service (absent with --no-modules).
    pub modules: Option<Arc<freergb_modules::setup::Manager>>,
    light_test: Option<(std::collections::BTreeSet<String>, Instant)>,
    workers: Vec<Worker>,
    spec: EffectSpec,
    effect: Box<dyn Effect>,
    effect_start: Instant,
    canvas: Canvas,
    brightness: f32,
    started: Instant,
    ticks: u64,
    target_fps: u32,
    fps_window: (Instant, u64, f64),
    state_path: Option<PathBuf>,
    /// Per-device brightness by device id (persisted).
    device_brightness: std::collections::BTreeMap<String, f32>,
    /// Per-device canvas regions by device id (persisted).
    device_regions: std::collections::BTreeMap<String, Region>,
    profiles_dir: Option<PathBuf>,
    retry_interval: Duration,
    conflicts: Vec<&'static str>,
    lighting_paused: bool,
    /// System-audio capture, running only while an audio effect is active.
    audio: Option<freergb_audio::AudioInput>,
    audio_error: Option<String>,
    /// Screen capture, running only while a screen effect is active.
    screen: Option<freergb_screen::ScreenInput>,
    screen_error: Option<String>,
}

struct PluginEntry {
    manifest: Manifest,
    enabled: bool,
    allowed: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub access: Access,
    pub enabled: bool,
    pub allowed: bool,
    pub status: Option<DeviceStatus>,
    pub last_error: Option<String>,
    pub frames: u64,
}

/// Body of a device worker thread.
fn run_worker(
    mut dev: Box<dyn RgbDevice>,
    view: Arc<Mutex<View>>,
    slot: Slot,
    stop: Arc<AtomicBool>,
    retry_interval: Duration,
) {
    let publish = |dev: &dyn RgbDevice, view: &Mutex<View>, err: Option<Option<String>>| {
        let mut v = guard(view);
        let status = dev.status();
        if v.status != status || v.layout != *dev.layout() {
            v.key_map = dev.key_map();
        }
        v.status = status;
        v.info = dev.info().clone();
        v.sensors = dev.sensors();
        v.sensor_snapshot_at = Instant::now();
        if v.layout != *dev.layout() {
            v.layout = dev.layout().clone();
        }
        if let Some(e) = err {
            v.last_error = e;
        }
    };

    // Exponential back-off: retry_interval, x2, x4 … capped at MAX_RETRY. A
    // hung device then blocks the shared HID gate only rarely.
    let mut failures: u32 = 0;
    let backoff = |failures: u32| {
        let factor = 1u32 << failures.saturating_sub(1).min(5);
        (retry_interval * factor).min(MAX_RETRY.max(retry_interval))
    };

    // Connect right away; on failure retry after the back-off.
    let mut next_retry = match dev.connect() {
        Ok(()) => {
            publish(dev.as_ref(), &view, Some(None));
            None
        }
        Err(e) => {
            tracing::warn!(device = %dev.info().id, error = %e, "connect failed");
            publish(dev.as_ref(), &view, Some(Some(e.to_string())));
            failures = 1;
            Some(Instant::now() + backoff(failures))
        }
    };

    while !stop.load(Ordering::SeqCst) {
        let frame = {
            let (lock, cv) = &*slot;
            let mut pending = guard(lock);
            if pending.is_none() {
                pending = cv
                    .wait_timeout(pending, WORKER_POLL)
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            }
            pending.take()
        };
        if stop.load(Ordering::SeqCst) {
            break;
        }

        match dev.status() {
            DeviceStatus::Connected => {
                let Some(frame) = frame else { continue };
                match dev.push_frame(&frame) {
                    Ok(()) => {
                        let mut view = guard(&view);
                        view.frames += 1;
                        view.sensors = dev.sensors();
                        view.sensor_snapshot_at = Instant::now();
                    }
                    Err(e) if e.is_gone() => {
                        tracing::info!(device = %dev.info().id, "device unplugged, releasing worker");
                        publish(dev.as_ref(), &view, Some(Some(e.to_string())));
                        return;
                    }
                    Err(e) => {
                        tracing::warn!(device = %dev.info().id, error = %e, "frame failed");
                        failures = 1;
                        next_retry = Some(Instant::now() + backoff(failures));
                        publish(dev.as_ref(), &view, Some(Some(e.to_string())));
                    }
                }
            }
            DeviceStatus::Disconnected | DeviceStatus::Faulted(_) => {
                let due = next_retry.get_or_insert_with(|| Instant::now() + retry_interval);
                if Instant::now() >= *due {
                    let _ = dev.disconnect(None);
                    match dev.connect() {
                        Err(e) if e.is_gone() => {
                            tracing::info!(device = %dev.info().id, "device unplugged, releasing worker");
                            publish(dev.as_ref(), &view, Some(Some(e.to_string())));
                            return;
                        }
                        Ok(()) => {
                            tracing::info!(device = %dev.info().id, "reconnected");
                            next_retry = None;
                            failures = 0;
                            publish(dev.as_ref(), &view, Some(None));
                        }
                        Err(e) => {
                            failures = failures.saturating_add(1);
                            let wait = backoff(failures);
                            tracing::debug!(device = %dev.info().id, error = %e, retry_in_s = wait.as_secs_f32(), "reconnect failed");
                            next_retry = Some(Instant::now() + wait);
                            publish(dev.as_ref(), &view, Some(Some(e.to_string())));
                        }
                    }
                }
            }
        }
    }

    if let Err(e) = dev.disconnect(None) {
        tracing::warn!(device = %dev.info().id, error = %e, "disconnect failed");
    }
    publish(dev.as_ref(), &view, None);
}

impl Engine {
    pub fn new(spec: EffectSpec, target_fps: u32) -> Self {
        let now = Instant::now();
        let mut engine = Self {
            marketplace: None,
            effect_fault: freergb_effect_host::runtime::WasmEffect::fault(),
            activation_error: None,
            channel_bindings: Default::default(),
            keyboard: Default::default(),
            controls: Default::default(),
            mainboard: Default::default(),
            hwmon: Default::default(),
            gpu: Default::default(),
            cooling_sessions: Default::default(),
            background: Default::default(),
            fan_support: Default::default(),
            cooling_profiles: Default::default(),
            onboarding: Default::default(),
            mainboard_rgb: Default::default(),
            cooling_assignments: Default::default(),
            recorded_test_job: None,
            sensor_refresh_at: None,
            plugins: Default::default(),
            modules: None,
            light_test: None,
            workers: Vec::new(),
            effect: spec.build(),
            spec,
            effect_start: now,
            canvas: Canvas::new(CANVAS_W, CANVAS_H),
            brightness: 1.0,
            started: now,
            ticks: 0,
            target_fps: target_fps.clamp(1, 240),
            fps_window: (now, 0, 0.0),
            state_path: None,
            device_brightness: Default::default(),
            device_regions: Default::default(),
            profiles_dir: None,
            retry_interval: RETRY_INTERVAL,
            conflicts: Vec::new(),
            lighting_paused: false,
            audio: None,
            audio_error: None,
            screen: None,
            screen_error: None,
        };
        if engine.spec.needs_audio() || engine.spec.needs_screen() {
            // Starts the capture the effect needs.
            let spec = engine.spec.clone();
            engine.set_effect(spec);
        }
        engine
    }

    /// Overrides the reconnect back-off (tests use a short one).
    pub fn with_retry_interval(mut self, d: Duration) -> Self {
        self.retry_interval = d;
        self
    }

    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }

    /// User pause (tray/GUI): behaves like a conflicting program, so devices
    /// are handed back to hardware mode and hot-plug leaves them alone.
    pub fn lighting_paused(&self) -> bool {
        self.lighting_paused
    }
    pub fn set_lighting_paused(&mut self, paused: bool) -> usize {
        self.lighting_paused = paused;
        self.persist();
        if paused {
            let released = self.release_hardware();
            let mut conflicts = self.conflicts.clone();
            if !conflicts.contains(&PAUSED_BY_USER) {
                conflicts.push(PAUSED_BY_USER);
            }
            self.conflicts = conflicts;
            released
        } else {
            self.conflicts.retain(|c| *c != PAUSED_BY_USER);
            0
        }
    }

    pub fn conflicts(&self) -> &[&'static str] {
        &self.conflicts
    }

    pub fn set_conflicts(&mut self, c: Vec<&'static str>) {
        self.conflicts = c;
    }

    /// Register an explicit startup allowlist. No executable path comes from IPC.
    pub fn configure_plugins(
        &mut self,
        manifests: Vec<Manifest>,
        hardware: bool,
    ) -> Result<(), String> {
        if !self.plugins.is_empty() {
            return Err("plugins already configured".into());
        }
        if manifests.len() > 16 {
            return Err("at most 16 plugins may be configured".into());
        }
        if manifests
            .iter()
            .filter(|p| p.access == Access::Hardware)
            .count()
            > 1
        {
            return Err("only one hardware plugin is allowed: cross-process HID serialization is unavailable".into());
        }
        let mut entries = std::collections::BTreeMap::new();
        for manifest in manifests {
            manifest.validate().map_err(|e| e.to_string())?;
            let allowed = hardware || manifest.access == Access::Virtual;
            let id = manifest.id.clone();
            if entries
                .insert(
                    id,
                    PluginEntry {
                        manifest,
                        enabled: allowed,
                        allowed,
                        error: None,
                    },
                )
                .is_some()
            {
                return Err("duplicate plugin id".into());
            }
        }
        self.plugins = entries;
        Ok(())
    }

    /// Hardware plugins run exclusively from built-in adapters: the HID gate is
    /// process-local and cannot serialize native plugin I/O.
    pub fn has_hardware_plugins(&self) -> bool {
        self.plugins
            .values()
            .any(|p| p.allowed && p.manifest.access == Access::Hardware)
    }

    pub fn plugins(&self) -> Vec<PluginSummary> {
        let devices = self.devices();
        self.plugins
            .iter()
            .map(|(id, p)| {
                let device = devices.iter().find(|d| d.info.id == p.manifest.device_id());
                PluginSummary {
                    id: id.clone(),
                    name: p.manifest.name.clone(),
                    access: p.manifest.access,
                    enabled: p.enabled,
                    allowed: p.allowed,
                    status: device.map(|d| d.status.clone()),
                    last_error: p
                        .error
                        .clone()
                        .or_else(|| device.and_then(|d| d.last_error.clone())),
                    frames: device.map_or(0, |d| d.frames),
                }
            })
            .collect()
    }

    pub fn set_plugin_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        let plugin = self
            .plugins
            .get_mut(id)
            .ok_or("plugin is not in the startup allowlist")?;
        if enabled && !plugin.allowed {
            return Err("hardware plugins disabled by --no-hardware".into());
        }
        plugin.enabled = enabled;
        plugin.error = None;
        let device_id = plugin.manifest.device_id();
        if !enabled {
            if let Some(index) = self.workers.iter().position(|w| w.id == device_id) {
                let mut worker = self.workers.remove(index);
                worker.stop.store(true, Ordering::SeqCst);
                worker.slot.1.notify_one();
                if let Some(handle) = worker.handle.take() {
                    let _ = handle.join();
                }
            }
        }
        Ok(())
    }

    fn sync_plugins(&mut self) {
        let pending: Vec<_> = self
            .plugins
            .values()
            .filter(|p| {
                p.enabled
                    && (p.manifest.access == Access::Virtual || self.conflicts.is_empty())
                    && !self.has_device(&p.manifest.device_id())
            })
            .map(|p| p.manifest.clone())
            .collect();
        for manifest in pending {
            let id = manifest.id.clone();
            match ProcessDevice::new(manifest) {
                Ok(device) => self.add_device(Box::new(device)),
                Err(error) => {
                    tracing::warn!(%error, "plugin configuration unavailable");
                    if let Some(plugin) = self.plugins.get_mut(&id) {
                        plugin.enabled = false;
                        plugin.error = Some(error.to_string());
                    }
                }
            }
        }
    }

    /// Stops and removes every real (`hid:`) device; each worker hands its
    /// device back to hardware mode first. Virtual devices stay. The next
    /// hot-plug scan without conflicts adds the hardware again.
    pub fn release_hardware(&mut self) -> usize {
        let (hw, keep): (Vec<Worker>, Vec<Worker>) = std::mem::take(&mut self.workers)
            .into_iter()
            .partition(|w| {
                w.id.0.starts_with("hid:")
                    || w.id.0.starts_with("module:")
                    || (w.id.0.starts_with("plugin:")
                        && guard(&w.view).info.kind != freergb_device_api::DeviceKind::Virtual)
            });
        self.workers = keep;
        let n = hw.len();
        for w in &hw {
            w.stop.store(true, Ordering::SeqCst);
            w.slot.1.notify_one();
        }
        for mut w in hw {
            if let Some(h) = w.handle.take() {
                let _ = h.join();
            }
        }
        n
    }

    /// Drops workers that have finished (device unplugged), so a hot-plug scan
    /// can add the device again with a fresh handle. Returns how many.
    pub fn prune_finished(&mut self) -> usize {
        let before = self.workers.len();
        self.workers.retain_mut(|w| {
            let done = w.handle.as_ref().is_some_and(JoinHandle::is_finished);
            if done {
                if let Some(h) = w.handle.take() {
                    let _ = h.join();
                }
                tracing::info!(device = %w.id, "removed unplugged device");
            }
            !done
        });
        before - self.workers.len()
    }

    /// Stops and removes the workers whose id satisfies `select`; each hands
    /// its device back first. Returns how many were removed.
    pub fn remove_devices(&mut self, select: impl Fn(&DeviceId) -> bool) -> usize {
        let (gone, keep): (Vec<Worker>, Vec<Worker>) = std::mem::take(&mut self.workers)
            .into_iter()
            .partition(|w| select(&w.id));
        self.workers = keep;
        for w in &gone {
            w.stop.store(true, Ordering::SeqCst);
            w.slot.1.notify_one();
        }
        let n = gone.len();
        for mut w in gone {
            if let Some(h) = w.handle.take() {
                let _ = h.join();
            }
        }
        n
    }

    /// Lets the given devices glow once, softly, for [`LIGHT_TEST`]. The
    /// running effect continues underneath and takes over again afterwards,
    /// so the previous lighting is restored exactly.
    pub fn start_light_test(&mut self, ids: Vec<String>) -> Result<(), String> {
        if ids.is_empty()
            || ids
                .iter()
                .any(|id| !self.workers.iter().any(|w| &w.id.0 == id))
        {
            return Err("light test needs connected devices".into());
        }
        self.light_test = Some((ids.into_iter().collect(), Instant::now()));
        Ok(())
    }

    /// Remaining light test time in seconds, if one runs.
    pub fn light_test_remaining(&self) -> Option<f64> {
        self.light_test
            .as_ref()
            .and_then(|(_, at)| LIGHT_TEST.checked_sub(at.elapsed()))
            .map(|d| d.as_secs_f64())
    }

    pub fn has_device(&self, id: &DeviceId) -> bool {
        self.workers.iter().any(|w| &w.id == id)
    }

    /// Hands the device to its own worker thread, which connects it and
    /// retries on failure.
    pub fn add_device(&mut self, dev: Box<dyn RgbDevice>) {
        let id = dev.info().id.clone();
        let id_str = id.0.clone();
        tracing::info!(device = %id, name = %dev.info().name, "device added");
        let view = Arc::new(Mutex::new(View {
            key_map: dev.key_map(),
            sensors: Vec::new(),
            sensor_snapshot_at: Instant::now(),
            info: dev.info().clone(),
            status: DeviceStatus::Disconnected,
            layout: dev.layout().clone(),
            frames: 0,
            dropped: 0,
            last_error: None,
        }));
        let slot: Slot = Arc::new((Mutex::new(None), Condvar::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let (view, slot, stop, retry) = (
                view.clone(),
                slot.clone(),
                stop.clone(),
                self.retry_interval,
            );
            std::thread::Builder::new()
                .name(format!("dev-{id}"))
                .spawn(move || run_worker(dev, view, slot, stop, retry))
                .expect("spawn device worker")
        };
        self.workers.push(Worker {
            id,
            view,
            slot,
            stop,
            handle: Some(handle),
            region: self
                .device_regions
                .get(&id_str)
                .copied()
                .unwrap_or(Region::FULL),
            correction: ColorCorrection {
                brightness: self.device_brightness.get(&id_str).copied().unwrap_or(1.0),
                ..ColorCorrection::default()
            },
        });
    }

    /// Restores effect and brightness from `path` and saves every later change
    /// there.
    pub fn with_state_file(mut self, path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            self.controls =
                crate::control::Controls::default().with_file(parent.join("control.json"));
            self.keyboard =
                std::mem::take(&mut self.keyboard).with_file(parent.join("keyboard-reactive.json"));
            self.channel_bindings = std::mem::take(&mut self.channel_bindings)
                .with_file(parent.join("cooling-channel-bindings.json"));
            self.background = std::mem::take(&mut self.background)
                .with_file(parent.join("cooling-background.json"));
            self.fan_support = std::mem::take(&mut self.fan_support)
                .with_file(parent.join("cooling-fan-support.json"));
            self.cooling_profiles = std::mem::take(&mut self.cooling_profiles)
                .with_file(parent.join("cooling-profiles.json"));
            self.onboarding =
                std::mem::take(&mut self.onboarding).with_file(parent.join("onboarding.json"));
            self.cooling_assignments = std::mem::take(&mut self.cooling_assignments)
                .with_file(parent.join("cooling-assignments.json"));
            self.mainboard_rgb = std::mem::take(&mut self.mainboard_rgb)
                .with_file(parent.join("mainboard-rgb.json"));
        }
        let s = state::load(&path);
        self.set_effect(s.effect);
        self.brightness = s.brightness.clamp(0.0, 1.0);
        self.device_brightness = s.device_brightness;
        self.device_regions = s.device_regions;
        self.lighting_paused = s.lighting_paused;
        self.profiles_dir = crate::profiles::dir_for_state(&path);
        self.state_path = Some(path);
        self
    }

    /// Directory for named profiles; `None` when state persistence is off.
    pub fn profiles_dir(&self) -> Option<&std::path::Path> {
        self.profiles_dir.as_deref()
    }

    /// Directory of the effect editor's library (`effects/` next to profiles).
    pub fn custom_effects_dir(&self) -> Option<std::path::PathBuf> {
        self.profiles_dir
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(|d| d.join("effects"))
    }

    /// Uses `dir` for named profiles (tests, or a custom location).
    pub fn with_profiles_dir(mut self, dir: PathBuf) -> Self {
        self.profiles_dir = Some(dir);
        self
    }

    pub fn current_state(&self) -> state::State {
        state::State {
            effect: self.spec.clone(),
            brightness: self.brightness,
            device_brightness: self.device_brightness.clone(),
            device_regions: self.device_regions.clone(),
            lighting_paused: self.lighting_paused,
        }
    }

    /// Applies a saved state (effect + brightness) and persists it.
    pub fn apply_state(&mut self, s: state::State) {
        self.brightness = s.brightness.clamp(0.0, 1.0);
        self.device_brightness = s.device_brightness;
        self.device_regions = s.device_regions;
        for w in &mut self.workers {
            w.correction.brightness = self.device_brightness.get(&w.id.0).copied().unwrap_or(1.0);
            w.region = self
                .device_regions
                .get(&w.id.0)
                .copied()
                .unwrap_or(Region::FULL);
        }
        self.set_effect(s.effect);
    }

    fn persist(&self) {
        if let Some(path) = &self.state_path {
            let s = self.current_state();
            if let Err(e) = state::save(path, &s) {
                tracing::warn!(path = %path.display(), error = %e, "saving state failed");
            }
        }
    }

    /// Starts or stops the audio capture to match `spec` and returns the
    /// context the effect is built with.
    fn audio_context(&mut self, spec: &EffectSpec) -> freergb_effects::EffectContext {
        if !spec.needs_audio() {
            if self.audio.take().is_some() {
                tracing::info!("audio capture stopped (no audio effect active)");
            }
            self.audio_error = None;
            return freergb_effects::EffectContext {
                audio: None,
                screen: self.screen_handle(spec),
            };
        }
        if self.audio.is_none() {
            match freergb_audio::AudioInput::start_system_audio() {
                Ok(a) => {
                    tracing::info!(device = %a.device_name(), "audio capture started for audio effect");
                    self.audio = Some(a);
                    self.audio_error = None;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "audio capture unavailable, audio effect stays dark");
                    self.audio_error = Some(e.to_string());
                }
            }
        }
        freergb_effects::EffectContext {
            audio: self.audio.as_ref().map(freergb_audio::AudioInput::shared),
            screen: self.screen_handle(spec),
        }
    }

    /// Starts or stops the screen capture to match `spec`.
    fn screen_handle(&mut self, spec: &EffectSpec) -> Option<freergb_effects::ScreenHandle> {
        if !spec.needs_screen() {
            if self.screen.take().is_some() {
                tracing::info!("screen capture stopped (no screen effect active)");
            }
            self.screen_error = None;
            return None;
        }
        if self.screen.is_none() {
            match freergb_screen::ScreenInput::start_primary(30) {
                Ok(s) => {
                    tracing::info!(monitor = %s.monitor_name(), "screen capture started for screen effect");
                    self.screen = Some(s);
                    self.screen_error = None;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "screen capture unavailable, screen effect stays dark");
                    self.screen_error = Some(e.to_string());
                }
            }
        }
        self.screen
            .as_ref()
            .map(freergb_screen::ScreenInput::shared)
    }

    pub fn runtime_error_code(&self) -> Option<&'static str> {
        if self.activation_error.is_some() {
            Some("FX_ACTIVATION_FAILED")
        } else if self
            .effect_fault
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
        {
            Some("FX_RUNTIME_FAULT")
        } else {
            None
        }
    }

    pub fn runtime_error(&self) -> Option<String> {
        self.activation_error.clone().or_else(|| {
            self.effect_fault
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        })
    }

    pub fn try_set_effect(&mut self, spec: EffectSpec) -> Result<(), String> {
        spec.validate()?;
        let fault = freergb_effect_host::runtime::WasmEffect::fault();
        let prepared = if let EffectSpec::Marketplace {
            id,
            version,
            parameters,
        } = &spec
        {
            let manager = self
                .marketplace
                .as_ref()
                .ok_or("Marketplace is unavailable")?;
            Some(Box::new(manager.load(id, version, parameters, fault.clone())?) as Box<dyn Effect>)
        } else {
            None
        };
        // Do not acknowledge an activation that cannot survive restart.
        if let Some(path) = &self.state_path {
            let mut state = self.current_state();
            state.effect = spec.clone();
            state::save(path, &state)
                .map_err(|e| format!("Effect was not activated: cannot save state ({e})"))?;
        }
        let ctx = self.audio_context(&spec);
        self.effect = prepared.unwrap_or_else(|| spec.build_with(&ctx));
        self.effect_fault = fault;
        self.activation_error = None;
        self.spec = spec;
        self.effect_start = Instant::now();
        Ok(())
    }

    pub fn set_effect(&mut self, spec: EffectSpec) {
        if let Err(error) = self.try_set_effect(spec) {
            tracing::warn!(%error, "effect activation failed");
            // A failed selection must not fault the still-running guest.
            self.activation_error = Some(error);
        }
    }

    pub fn effect(&self) -> &EffectSpec {
        &self.spec
    }

    /// Places one device on the canvas. Returns `false` if no such device;
    /// the caller validates the region.
    pub fn set_device_region(&mut self, id: &str, region: Region) -> bool {
        let Some(w) = self.workers.iter_mut().find(|w| w.id.0 == id) else {
            return false;
        };
        w.region = region;
        self.device_regions.insert(id.to_owned(), region);
        self.persist();
        true
    }

    /// Sets one device's brightness (0..=1). Returns `false` if no such device.
    pub fn set_device_brightness(&mut self, id: &str, b: f32) -> bool {
        let b = b.clamp(0.0, 1.0);
        let Some(w) = self.workers.iter_mut().find(|w| w.id.0 == id) else {
            return false;
        };
        w.correction.brightness = b;
        self.device_brightness.insert(id.to_owned(), b);
        self.persist();
        true
    }

    pub fn set_brightness(&mut self, b: f32) {
        self.brightness = b.clamp(0.0, 1.0);
        self.persist();
    }

    /// Renders one frame at `now` and offers it to every connected device.
    /// Never blocks on device I/O.
    pub fn tick(&mut self, now: Instant) {
        self.record_channel_test();
        let has_keyboard = self.conflicts.is_empty()
            && self.workers.iter().any(|w| {
                let view = guard(&w.view);
                view.status == DeviceStatus::Connected
                    && view.info.kind == freergb_device_api::DeviceKind::Keyboard
                    && !view.key_map.is_empty()
                    && self.keyboard.config.devices.contains(&w.id.0)
            });
        self.keyboard.tick(has_keyboard);
        if self
            .sensor_refresh_at
            .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_secs(1))
        {
            self.sensor_refresh_at = Some(now);
            let mut devices = serde_json::json!(self.devices());
            // LHM (CPU/GPU/board/storage) and Linux hwmon (k10temp, coretemp,
            // amdgpu, nvme, drivetemp, SuperIO) all feed curves and custom sensors.
            for entry in self
                .mainboard
                .sensors()
                .into_iter()
                .chain(self.hwmon.sensors())
            {
                devices.as_array_mut().unwrap().push(serde_json::json!({
                    "status":"Connected", "provider":entry.provider, "info":{"id":entry.device_id,"name":entry.device_name}, "sensors":[entry.sensor]
                }));
            }
            self.controls.refresh_sensors(&devices);
            self.gpu.tick(&self.controls.composite_feed, now);
        }
        if let Some(step) = self.controls.tick(now) {
            use freergb_control::MacroStep;
            match step {
                MacroStep::Color { color } => self.set_effect(EffectSpec::Solid { color }),
                MacroStep::Brightness { value } => self.set_brightness(value),
                MacroStep::Effect { effect } => self.set_effect(effect),
                MacroStep::Text { .. } | MacroStep::Key { .. } | MacroStep::Chord { .. } => {
                    if let Err(error) = crate::control::input(&step) {
                        self.controls.fail(error);
                    }
                }
                MacroStep::Delay { .. } => {}
            }
        }
        self.sync_plugins();
        self.effect.render(
            now.saturating_duration_since(self.effect_start),
            &mut self.canvas,
        );
        let brightness = self.brightness;
        if self
            .light_test
            .as_ref()
            .is_some_and(|(_, at)| now.saturating_duration_since(*at) >= LIGHT_TEST)
        {
            self.light_test = None;
        }
        let test_color = self.light_test.as_ref().map(|(ids, at)| {
            let t = now.saturating_duration_since(*at).as_secs_f32() / LIGHT_TEST.as_secs_f32();
            let level = (std::f32::consts::PI * t).sin().powi(2);
            let v = (level * 255.0).round() as u8;
            (ids, freergb_core::Rgb { r: v, g: v, b: v })
        });

        for w in &self.workers {
            let (layout, key_map) = {
                let v = guard(&w.view);
                if v.status != DeviceStatus::Connected {
                    continue;
                }
                (v.layout.clone(), v.key_map.clone())
            };
            let corr = ColorCorrection {
                brightness: w.correction.brightness * brightness,
                ..w.correction
            };
            // Some controllers discover their layout on the worker's first
            // connect. Do not queue an empty frame using the pre-discovery view.
            if layout.is_empty() {
                continue;
            }
            let mut frame = self.canvas.sample_layout(&layout, w.region);
            self.keyboard.apply(&w.id.0, &key_map, &mut frame);
            if let Some((ids, color)) = &test_color {
                if ids.contains(&w.id.0) {
                    frame = Frame::solid(layout.len(), *color);
                }
            }
            let frame = frame.corrected(&corr);
            let (lock, cv) = &*w.slot;
            if guard(lock).replace(frame).is_some() {
                guard(&w.view).dropped += 1;
            }
            cv.notify_one();
        }

        self.ticks += 1;
        let (since, ticks_then, _) = self.fps_window;
        let elapsed = now.saturating_duration_since(since);
        if elapsed >= Duration::from_secs(1) {
            let fps = (self.ticks - ticks_then) as f64 / elapsed.as_secs_f64();
            self.fps_window = (now, self.ticks, fps);
        }
    }

    pub fn devices(&self) -> Vec<DeviceSummary> {
        self.workers
            .iter()
            .map(|w| {
                let v = guard(&w.view);
                DeviceSummary {
                    sensors: v
                        .sensors
                        .iter()
                        .cloned()
                        .map(|mut sensor| {
                            sensor.age_ms = sensor.age_ms.saturating_add(
                                v.sensor_snapshot_at
                                    .elapsed()
                                    .as_millis()
                                    .min(u128::from(u64::MAX))
                                    as u64,
                            );
                            sensor
                        })
                        .collect(),
                    info: v.info.clone(),
                    status: v.status.clone(),
                    brightness: w.correction.brightness,
                    region: w.region,
                    leds: v.layout.len(),
                    frames: v.frames,
                    dropped: v.dropped,
                    last_error: v.last_error.clone(),
                }
            })
            .collect()
    }

    pub fn keyboard_reactive(&self) -> serde_json::Value {
        let devices: Vec<_> = self.workers.iter().filter_map(|worker| {
            let view = guard(&worker.view);
            (view.info.kind == freergb_device_api::DeviceKind::Keyboard).then(|| serde_json::json!({
                "id":worker.id.0,"name":view.info.name,"connected":view.status == DeviceStatus::Connected,
                "keys":view.key_map,"layout":view.layout,
            }))
        }).collect();
        serde_json::json!({"config":self.keyboard.config,"status":self.keyboard.status(),"devices":devices})
    }

    /// Resolve the support tier and user evidence for a native control id from
    /// the protected inventory and bindings (never from the GUI request).
    /// Current hardware identity of a native control for assignment checks.
    pub fn cooling_identity(
        &self,
        control_id: &str,
    ) -> Option<crate::cooling_assignments::Identity> {
        let channels = self.mainboard.channels();
        crate::cooling_assignments::identity_of(control_id, &channels, |id| {
            self.channel_bindings.binding(id)
        })
    }

    /// Translates `mode:"rpm"` into a duty from this channel's current
    /// calibration or channel-test measurements (recomputed on every start).
    fn resolve_rpm_target(
        &self,
        request: &mut crate::cooling_sessions::StartRequest,
    ) -> Result<(), String> {
        if request.mode != "rpm" {
            return Ok(());
        }
        let target = request.target_rpm.ok_or("mode=rpm requires target_rpm")?;
        let identity = self
            .channel_identity(&request.control_id)
            .ok_or("Unknown channel identity; RPM targets need a measured channel")?;
        let duty = self
            .fan_support
            .rpm_map(&identity)?
            .duty_for(target as f32)?;
        request.rpm_duty = Some(duty.ceil().clamp(0.0, 100.0) as u8);
        Ok(())
    }

    /// Starts a live cooling job through the normal policy and remembers
    /// continuous assignments for the next daemon start.
    pub fn start_cooling(
        &mut self,
        mut request: crate::cooling_sessions::StartRequest,
    ) -> Result<u64, String> {
        request.composite_feed = Some(self.controls.composite_feed.clone());
        self.resolve_rpm_target(&mut request)?;
        let policy = self.cooling_policy(&request.control_id)?;
        if request.mode == "composite"
            && !request
                .composite_output
                .as_deref()
                .is_some_and(|o| self.controls.has_composite_output(o))
        {
            return Err(
                "Choose a saved composite curve (Cooling → Advanced → Composite curves)".into(),
            );
        }
        let remembered = crate::cooling_assignments::persistable(&request)
            .then(|| (request.clone(), self.cooling_identity(&request.control_id)));
        let job = self.cooling_sessions.start(request, &policy)?;
        if let Some((request, Some(identity))) = remembered {
            self.cooling_assignments.remember(request, identity);
        }
        Ok(job)
    }

    /// Resumes remembered assignments once the channel inventory is known
    /// (called periodically; does nothing after the first decision).
    pub fn resume_cooling(&mut self) {
        if !self.cooling_sessions.status().available {
            return;
        }
        let ready = !self.mainboard.channels().is_empty();
        let identities: std::collections::BTreeMap<_, _> = self
            .cooling_assignments
            .control_ids()
            .into_iter()
            .filter_map(|id| self.cooling_identity(&id).map(|i| (id, i)))
            .collect();
        let requests = self
            .cooling_assignments
            .take_resumable(|id| identities.get(id).cloned(), ready);
        for mut request in requests {
            request.composite_feed = Some(self.controls.composite_feed.clone());
            let control = request.control_id.clone();
            let result = self
                .resolve_rpm_target(&mut request)
                .and_then(|()| self.cooling_policy(&control))
                .and_then(|policy| self.cooling_sessions.start(request, &policy));
            match result {
                Ok(job) => tracing::info!(job, control, "resumed cooling assignment"),
                Err(error) => {
                    tracing::warn!(%error, control, "cooling assignment not resumed");
                    self.cooling_assignments.flag(&control, error);
                }
            }
        }
    }

    pub fn cooling_policy(
        &self,
        control_id: &str,
    ) -> Result<crate::cooling_sessions::SessionPolicy, String> {
        use freergb_control::fan_support::{ChannelEvidence, SupportTier};
        // Every fan write (sessions, tests, background, profiles) passes here.
        self.onboarding.require_safety()?;
        let channels = self.mainboard.channels();
        let Some(channel) = channels
            .iter()
            .find(|c| c.kind == "control" && c.sensor_id == control_id)
        else {
            // Unknown to the cached inventory: the protected worker still checks
            // the identity; legacy rules (verified readback required) apply.
            return Ok(crate::cooling_sessions::SessionPolicy::legacy());
        };
        let capability = match channel.support.as_ref() {
            // Older protected services report no support: verified-only as before.
            None => SupportTier::Verified,
            Some(support) if support.software_mode => {
                return Err("This channel is already in software mode (another tool controls it); freeRGB will not take it over".into())
            }
            Some(support) if support.held_full => {
                return Err("This channel is held at 100% after a worker crash; restart Windows to return it to firmware first".into())
            }
            Some(support) if support.tier == "verified" => SupportTier::Verified,
            Some(_) => SupportTier::LhmSupported,
        };
        let identity = self.channel_identity(control_id);
        let unlock = identity.as_ref().and_then(|i| self.fan_support.unlock(i));
        Ok(crate::cooling_sessions::SessionPolicy {
            capability,
            evidence: ChannelEvidence {
                role: self.channel_bindings.role(&channel.id),
                test_passed: identity
                    .as_ref()
                    .is_some_and(|i| self.fan_support.passed_test(i).is_some()),
                unlocked: unlock.is_some(),
                pump_confirmed: unlock.is_some_and(|u| u.pump_confirmed),
            },
        })
    }

    /// Board/firmware/channel identity of a native control, if the protected
    /// inventory reports the board.
    pub fn channel_identity(
        &self,
        control_id: &str,
    ) -> Option<crate::fan_support::ChannelIdentity> {
        let board = self.mainboard.summary().board;
        self.mainboard
            .channels()
            .iter()
            .find(|c| c.kind == "control" && c.sensor_id == control_id)
            .and_then(|c| crate::fan_support::ChannelIdentity::of(board.as_ref(), c))
    }

    /// Tested duty/RPM pairs of a passed channel test for this exact identity.
    pub fn plausibility(&self, control_id: &str) -> Option<crate::background::Plausibility> {
        let identity = self.channel_identity(control_id)?;
        self.fan_support
            .passed_test(&identity)
            .map(|t| t.tested.iter().map(|d| (d.duty_percent, d.rpm)).collect())
    }

    /// Unlock (or confirm pump control for) a channel after the typed confirmation.
    pub fn unlock_channel(
        &mut self,
        control_id: &str,
        typed: &str,
        pump: bool,
    ) -> Result<(), String> {
        self.onboarding.require_safety()?;
        let channels = self.mainboard.channels();
        let channel = channels
            .iter()
            .find(|c| c.kind == "control" && c.sensor_id == control_id && c.control_available)
            .ok_or("Unknown or uncontrollable channel")?;
        if channel.support.as_ref().is_some_and(|s| s.software_mode) {
            return Err(
                "This channel is already in software mode (another tool); it cannot be unlocked"
                    .into(),
            );
        }
        let identity = self.channel_identity(control_id).ok_or(
            "The protected service reports no board identity; update the mainboard service",
        )?;
        if pump {
            if self.channel_bindings.role(&channel.id)
                != Some(freergb_control::channels::ChannelRole::Pump)
            {
                return Err("Pump confirmation applies only to channels bound as pump".into());
            }
            self.fan_support.confirm_pump(&identity, typed)
        } else {
            self.fan_support
                .unlock_channel(identity, &channel.name, typed)
        }
    }

    /// Validate every channel of a cooling profile under the current policy
    /// (tier, test, unlock) and hand it to the protected service (UAC). Unlike a
    /// revocation re-apply, a channel that is no longer permitted fails the
    /// activation instead of being dropped silently.
    pub fn activate_profile(&mut self, name: &str) -> Result<(), String> {
        let request = self
            .cooling_profiles
            .get(name)
            .cloned()
            .ok_or("Unknown cooling profile")?;
        let mut entries = Vec::new();
        for channel in &request.channels {
            let policy = self.cooling_policy(&channel.control_id)?;
            let bands = self.plausibility(&channel.control_id);
            entries.push(
                crate::background::protected_entry(channel, &policy, bands.as_ref())
                    .map_err(|e| format!("{}: {e}", channel.control_id))?,
            );
        }
        self.background.apply(request, entries)?;
        self.cooling_profiles.mark_active(name)
    }

    /// After a revocation, re-validate the saved background channels under the
    /// new policy, drop those no longer permitted and hand the result to the
    /// protected service (UAC). Returns whether a re-apply was started.
    pub fn reapply_background(&mut self) -> bool {
        let saved = self.background.view()["saved"].clone();
        let Ok(request) = serde_json::from_value::<crate::background::BackgroundRequest>(saved)
        else {
            return false;
        };
        if request.channels.is_empty() {
            return false;
        }
        let mut kept = crate::background::BackgroundRequest::default();
        let mut entries = Vec::new();
        for channel in request.channels {
            let Ok(policy) = self.cooling_policy(&channel.control_id) else {
                continue;
            };
            let bands = self.plausibility(&channel.control_id);
            if let Ok(entry) = crate::background::protected_entry(&channel, &policy, bands.as_ref())
            {
                entries.push(entry);
                kept.channels.push(channel);
            }
        }
        self.background.apply(kept, entries).is_ok()
    }

    /// Store a finished guided channel test once (bound to its identity).
    fn record_channel_test(&mut self) {
        let status = self.cooling_sessions.status();
        if status.state != "completed"
            || !matches!(status.mode.as_deref(), Some("channel_test" | "calibration"))
            || status.job_id.is_none()
            || status.job_id == self.recorded_test_job
        {
            return;
        }
        self.recorded_test_job = status.job_id;
        let (Some(control_id), Some(result)) = (
            status.control_id.as_deref(),
            status.report.as_ref().and_then(|r| r.calibration.as_ref()),
        ) else {
            return;
        };
        if let Some(identity) = self.channel_identity(control_id) {
            let recorded = if status.mode.as_deref() == Some("calibration") {
                self.fan_support.record_calibration(identity, result)
            } else {
                self.fan_support.record_test(identity, result)
            };
            if let Err(error) = recorded {
                tracing::warn!(%error, "channel test result not saved");
            }
        }
    }

    /// All cached sensor providers, without introducing non-RGB devices into devices.list.
    pub fn monitoring_sensors(&self) -> Vec<crate::mainboard::SensorEntry> {
        let mut entries = self.mainboard.sensors();
        entries.extend(self.hwmon.sensors());
        for device in self.devices() {
            for mut sensor in device.sensors {
                if device.status != DeviceStatus::Connected {
                    sensor.value = None;
                }
                entries.push(crate::mainboard::SensorEntry {
                    provider: "device",
                    device_id: device.info.id.0.clone(),
                    device_name: device.info.name.clone(),
                    sensor,
                });
            }
        }
        entries
    }

    pub fn diagnostics(&self) -> Diagnostics {
        Diagnostics {
            audio: match (&self.audio, &self.audio_error) {
                (Some(a), _) => Some(format!("capturing: {}", a.device_name())),
                (None, Some(e)) => Some(format!("error: {e}")),
                _ => None,
            },
            screen: match (&self.screen, &self.screen_error) {
                (Some(s), _) => Some(format!("capturing: {}", s.monitor_name())),
                (None, Some(e)) => Some(format!("error: {e}")),
                _ => None,
            },
            conflicts: self.conflicts.clone(),
            uptime_s: self.started.elapsed().as_secs_f64(),
            ticks: self.ticks,
            target_fps: self.target_fps,
            measured_fps: self.fps_window.2,
            effect: self.spec.clone(),
            brightness: self.brightness,
            devices: self.workers.len(),
            cooling_support: self
                .mainboard
                .channels()
                .into_iter()
                .filter(|c| c.kind == "control" && c.control_available)
                .map(|c| {
                    let support = c.support.clone();
                    CoolingSupportEntry {
                        name: c.name,
                        tier: support
                            .as_ref()
                            .map_or("unknown".into(), |s| s.tier.clone()),
                        register_readback: support.as_ref().is_some_and(|s| s.register_readback),
                        software_mode: support.as_ref().is_some_and(|s| s.software_mode),
                        held_full: support.as_ref().is_some_and(|s| s.held_full),
                    }
                })
                .collect(),
        }
    }

    /// Stops every worker; each releases its device back to hardware mode.
    /// Waits until all workers have finished.
    pub fn shutdown(&mut self) {
        self.keyboard.stop();
        self.hwmon.shutdown();
        for w in &self.workers {
            w.stop.store(true, Ordering::SeqCst);
            w.slot.1.notify_one();
        }
        for w in &mut self.workers {
            if let Some(h) = w.handle.take() {
                let _ = h.join();
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::Rgb;
    use freergb_device_api::{DeviceError, VirtualDevice};

    /// Polls `cond` for up to two seconds.
    fn wait_until(mut cond: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if cond() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn renders_to_virtual_device_with_brightness() {
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::WHITE }, 30);
        let dev = VirtualDevice::new("v", LedLayout::strip(4));
        let log = dev.frame_log();
        e.add_device(Box::new(dev));
        assert!(wait_until(
            || e.devices()[0].status == DeviceStatus::Connected
        ));

        e.tick(Instant::now());
        assert!(wait_until(|| log.total() == 1));
        assert_eq!(log.last().unwrap().colors(), &[Rgb::WHITE; 4]);

        e.set_brightness(0.5);
        e.set_effect(EffectSpec::Solid { color: Rgb::RED });
        e.tick(Instant::now());
        assert!(wait_until(|| log.total() == 2));
        assert_eq!(log.last().unwrap().colors(), &[Rgb::new(128, 0, 0); 4]);
        assert_eq!(e.devices()[0].frames, 2);
        e.shutdown();
        assert_eq!(e.devices()[0].status, DeviceStatus::Disconnected);
    }

    #[test]
    fn keyboard_overlay_targets_keys_respects_brightness_and_returns_to_base() {
        struct Keyboard {
            inner: VirtualDevice,
            info: DeviceInfo,
        }
        impl RgbDevice for Keyboard {
            fn info(&self) -> &DeviceInfo {
                &self.info
            }
            fn key_map(&self) -> std::collections::BTreeMap<u16, usize> {
                [(4, 0), (5, 1)].into()
            }
            fn layout(&self) -> &LedLayout {
                self.inner.layout()
            }
            fn connect(&mut self) -> Result<(), DeviceError> {
                self.inner.connect()
            }
            fn push_frame(&mut self, frame: &Frame) -> Result<(), DeviceError> {
                self.inner.push_frame(frame)
            }
            fn status(&self) -> DeviceStatus {
                self.inner.status()
            }
            fn disconnect(&mut self, color: Option<Rgb>) -> Result<(), DeviceError> {
                self.inner.disconnect(color)
            }
        }
        let inner = VirtualDevice::new("keyboard", LedLayout::strip(2));
        let log = inner.frame_log();
        let mut info = inner.info().clone();
        info.kind = freergb_device_api::DeviceKind::Keyboard;
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::BLUE }, 30);
        e.keyboard
            .configure(freergb_effects::keyboard::ReactiveConfig {
                fade_ms: 50,
                color: Rgb::RED,
                devices: [info.id.0.clone()].into(),
                ..Default::default()
            })
            .unwrap();
        e.add_device(Box::new(Keyboard { inner, info }));
        assert!(wait_until(
            || e.devices()[0].status == DeviceStatus::Connected
        ));
        e.keyboard.preview(4).unwrap();
        e.tick(Instant::now());
        assert!(wait_until(|| log.total() == 1));
        let frame = log.last().unwrap();
        assert!(frame.colors()[0].r > 0);
        assert_eq!(frame.colors()[1], Rgb::BLUE);
        assert_eq!(e.keyboard.status()["state"], "disabled"); // Preview installs no input hook.
        e.set_brightness(0.0);
        e.tick(Instant::now());
        assert!(wait_until(|| log.total() == 2));
        assert_eq!(log.last().unwrap(), Frame::solid(2, Rgb::BLACK));
        std::thread::sleep(Duration::from_millis(60));
        e.set_brightness(1.0);
        e.tick(Instant::now());
        assert!(wait_until(|| log.total() == 3));
        assert_eq!(log.last().unwrap(), Frame::solid(2, Rgb::BLUE));
    }

    /// Fails the first `fail_connects` connects; optionally sleeps per frame.
    struct Flaky {
        inner: VirtualDevice,
        fail_connects: u32,
        frame_delay: Duration,
    }

    impl RgbDevice for Flaky {
        fn info(&self) -> &DeviceInfo {
            self.inner.info()
        }
        fn connect(&mut self) -> Result<(), DeviceError> {
            if self.fail_connects > 0 {
                self.fail_connects -= 1;
                return Err(DeviceError::Protocol("simulated".into()));
            }
            self.inner.connect()
        }
        fn layout(&self) -> &LedLayout {
            self.inner.layout()
        }
        fn push_frame(&mut self, f: &Frame) -> Result<(), DeviceError> {
            std::thread::sleep(self.frame_delay);
            self.inner.push_frame(f)
        }
        fn status(&self) -> DeviceStatus {
            self.inner.status()
        }
        fn disconnect(&mut self, c: Option<Rgb>) -> Result<(), DeviceError> {
            self.inner.disconnect(c)
        }
    }

    #[test]
    fn retries_failed_connect_after_backoff() {
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::BLUE }, 30)
            .with_retry_interval(Duration::from_millis(50));
        let inner = VirtualDevice::new("flaky", LedLayout::strip(2));
        let log = inner.frame_log();
        e.add_device(Box::new(Flaky {
            inner,
            fail_connects: 1,
            frame_delay: Duration::ZERO,
        }));
        assert!(wait_until(|| e.devices()[0].last_error.is_some()));
        assert!(wait_until(
            || e.devices()[0].status == DeviceStatus::Connected
        ));
        assert_eq!(e.devices()[0].last_error, None);
        e.tick(Instant::now());
        assert!(wait_until(|| log.total() == 1));
    }

    /// Connects, then reports "unplugged" on the first frame.
    struct Unplugged(VirtualDevice);

    impl RgbDevice for Unplugged {
        fn info(&self) -> &DeviceInfo {
            self.0.info()
        }
        fn connect(&mut self) -> Result<(), DeviceError> {
            self.0.connect()
        }
        fn layout(&self) -> &LedLayout {
            self.0.layout()
        }
        fn push_frame(&mut self, _f: &Frame) -> Result<(), DeviceError> {
            Err(DeviceError::Transport(
                freergb_device_api::TransportError::Disconnected,
            ))
        }
        fn status(&self) -> DeviceStatus {
            self.0.status()
        }
        fn disconnect(&mut self, c: Option<Rgb>) -> Result<(), DeviceError> {
            self.0.disconnect(c)
        }
    }

    #[test]
    fn unplugged_device_is_pruned_and_can_be_re_added() {
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::RED }, 30);
        e.add_device(Box::new(Unplugged(VirtualDevice::new(
            "plug",
            LedLayout::strip(1),
        ))));
        let id = e.devices()[0].info.id.clone();
        assert!(wait_until(
            || e.devices()[0].status == DeviceStatus::Connected
        ));
        e.tick(Instant::now());
        assert!(
            wait_until(|| e.prune_finished() == 1),
            "worker should end after unplug"
        );
        assert!(!e.has_device(&id));
        e.add_device(Box::new(VirtualDevice::new("plug", LedLayout::strip(1))));
        assert!(e.has_device(&id));
        assert!(wait_until(
            || e.devices()[0].status == DeviceStatus::Connected
        ));
    }

    #[test]
    fn release_hardware_keeps_virtual_devices() {
        struct Hw(VirtualDevice, DeviceInfo);
        impl RgbDevice for Hw {
            fn info(&self) -> &DeviceInfo {
                &self.1
            }
            fn connect(&mut self) -> Result<(), DeviceError> {
                self.0.connect()
            }
            fn layout(&self) -> &LedLayout {
                self.0.layout()
            }
            fn push_frame(&mut self, f: &Frame) -> Result<(), DeviceError> {
                self.0.push_frame(f)
            }
            fn status(&self) -> DeviceStatus {
                self.0.status()
            }
            fn disconnect(&mut self, c: Option<Rgb>) -> Result<(), DeviceError> {
                self.0.disconnect(c)
            }
        }
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::RED }, 30);
        let inner = VirtualDevice::new("kb", LedLayout::strip(1));
        let mut info = inner.info().clone();
        info.id = DeviceId("hid:1b1c:1bfd:test".into());
        e.add_device(Box::new(Hw(inner, info)));
        e.add_device(Box::new(VirtualDevice::new("virt", LedLayout::strip(1))));
        assert!(wait_until(|| e
            .devices()
            .iter()
            .all(|d| d.status == DeviceStatus::Connected)));
        assert_eq!(e.release_hardware(), 1);
        assert_eq!(e.devices().len(), 1);
        assert_eq!(e.devices()[0].info.id.0, "virtual:virt");
    }

    #[test]
    fn lighting_pause_acts_like_a_conflict_and_resumes() {
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::RED }, 30);
        e.add_device(Box::new(VirtualDevice::new("virt", LedLayout::strip(1))));
        assert_eq!(e.set_lighting_paused(true), 0, "virtual devices stay");
        assert!(e.lighting_paused());
        assert_eq!(e.conflicts(), [PAUSED_BY_USER]);
        e.set_lighting_paused(true);
        assert_eq!(e.conflicts().len(), 1, "no duplicate pause entry");
        e.set_lighting_paused(false);
        assert!(!e.lighting_paused() && e.conflicts().is_empty());
    }

    #[test]
    fn slow_device_does_not_slow_down_fast_device() {
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::GREEN }, 60);
        let fast = VirtualDevice::new("fast", LedLayout::strip(2));
        let fast_log = fast.frame_log();
        let slow_inner = VirtualDevice::new("slow", LedLayout::strip(2));
        let slow_log = slow_inner.frame_log();
        e.add_device(Box::new(fast));
        e.add_device(Box::new(Flaky {
            inner: slow_inner,
            fail_connects: 0,
            frame_delay: Duration::from_millis(100),
        }));
        assert!(wait_until(|| e
            .devices()
            .iter()
            .all(|d| d.status == DeviceStatus::Connected)));

        let start = Instant::now();
        for _ in 0..20 {
            e.tick(Instant::now());
            std::thread::sleep(Duration::from_millis(10));
        }
        let tick_time = start.elapsed();
        assert!(
            wait_until(|| fast_log.total() >= 15),
            "fast got {}",
            fast_log.total()
        );
        // 20 ticks in ~200 ms: the 100 ms device can take only a few and
        // drops the rest, without stalling the ticks.
        assert!(
            tick_time < Duration::from_millis(600),
            "ticks blocked: {tick_time:?}"
        );
        assert!(slow_log.total() <= 5, "slow got {}", slow_log.total());
        assert!(e.devices()[1].dropped > 0);
        e.shutdown();
    }
}
#[test]
fn plugin_allowlist_is_atomic_and_no_hardware_is_enforced() {
    let mut engine = Engine::new(
        EffectSpec::Solid {
            color: freergb_core::Rgb::BLACK,
        },
        30,
    );
    let manifest = Manifest {
        api_version: 1,
        id: "blocked".into(),
        name: "Blocked hardware".into(),
        executable: std::env::current_exe().unwrap(),
        args: vec![],
        access: Access::Hardware,
        layout: LedLayout::strip(1),
        timeout_ms: 100,
    };
    assert!(engine
        .configure_plugins(vec![manifest.clone(), manifest.clone()], false)
        .is_err());
    assert!(engine.plugins().is_empty());
    engine.configure_plugins(vec![manifest], false).unwrap();
    assert!(!engine.plugins()[0].allowed);
    assert!(engine.set_plugin_enabled("blocked", true).is_err());
    assert!(engine.set_plugin_enabled("unknown", true).is_err());
    engine.tick(Instant::now());
    assert!(engine.devices().is_empty());
    assert!(!engine.has_hardware_plugins());
}
