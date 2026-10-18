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
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const RETRY_INTERVAL: Duration = Duration::from_secs(2);
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
    pub info: DeviceInfo,
    pub status: DeviceStatus,
    /// Per-device brightness (multiplied with the global brightness).
    pub brightness: f32,
    pub leds: usize,
    pub frames: u64,
    /// Frames replaced before the device could take them (device slower than the engine).
    pub dropped: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    pub uptime_s: f64,
    pub ticks: u64,
    pub target_fps: u32,
    pub measured_fps: f64,
    pub effect: EffectSpec,
    pub brightness: f32,
    pub devices: usize,
}

pub struct Engine {
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
    profiles_dir: Option<PathBuf>,
    retry_interval: Duration,
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
        v.status = dev.status();
        v.info = dev.info().clone();
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
                    Ok(()) => guard(&view).frames += 1,
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
        Self {
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
            profiles_dir: None,
            retry_interval: RETRY_INTERVAL,
        }
    }

    /// Overrides the reconnect back-off (tests use a short one).
    pub fn with_retry_interval(mut self, d: Duration) -> Self {
        self.retry_interval = d;
        self
    }

    pub fn target_fps(&self) -> u32 {
        self.target_fps
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
            region: Region::FULL,
            correction: ColorCorrection {
                brightness: self.device_brightness.get(&id_str).copied().unwrap_or(1.0),
                ..ColorCorrection::default()
            },
        });
    }

    /// Restores effect and brightness from `path` and saves every later change
    /// there.
    pub fn with_state_file(mut self, path: PathBuf) -> Self {
        let s = state::load(&path);
        self.set_effect(s.effect);
        self.brightness = s.brightness.clamp(0.0, 1.0);
        self.device_brightness = s.device_brightness;
        self.profiles_dir = crate::profiles::dir_for_state(&path);
        self.state_path = Some(path);
        self
    }

    /// Directory for named profiles; `None` when state persistence is off.
    pub fn profiles_dir(&self) -> Option<&std::path::Path> {
        self.profiles_dir.as_deref()
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
        }
    }

    /// Applies a saved state (effect + brightness) and persists it.
    pub fn apply_state(&mut self, s: state::State) {
        self.brightness = s.brightness.clamp(0.0, 1.0);
        self.device_brightness = s.device_brightness;
        for w in &mut self.workers {
            w.correction.brightness = self.device_brightness.get(&w.id.0).copied().unwrap_or(1.0);
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

    pub fn set_effect(&mut self, spec: EffectSpec) {
        self.effect = spec.build();
        self.spec = spec;
        self.effect_start = Instant::now();
        self.persist();
    }

    pub fn effect(&self) -> &EffectSpec {
        &self.spec
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
        self.effect.render(
            now.saturating_duration_since(self.effect_start),
            &mut self.canvas,
        );
        let brightness = self.brightness;

        for w in &self.workers {
            let layout = {
                let v = guard(&w.view);
                if v.status != DeviceStatus::Connected {
                    continue;
                }
                v.layout.clone()
            };
            let corr = ColorCorrection {
                brightness: w.correction.brightness * brightness,
                ..w.correction
            };
            let frame = self
                .canvas
                .sample_layout(&layout, w.region)
                .corrected(&corr);
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
                    info: v.info.clone(),
                    status: v.status.clone(),
                    brightness: w.correction.brightness,
                    leds: v.layout.len(),
                    frames: v.frames,
                    dropped: v.dropped,
                    last_error: v.last_error.clone(),
                }
            })
            .collect()
    }

    pub fn diagnostics(&self) -> Diagnostics {
        Diagnostics {
            uptime_s: self.started.elapsed().as_secs_f64(),
            ticks: self.ticks,
            target_fps: self.target_fps,
            measured_fps: self.fps_window.2,
            effect: self.spec.clone(),
            brightness: self.brightness,
            devices: self.workers.len(),
        }
    }

    /// Stops every worker; each releases its device back to hardware mode.
    /// Waits until all workers have finished.
    pub fn shutdown(&mut self) {
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
