//! Render engine: one effect draws onto a shared canvas, every device samples
//! its region, applies its colour correction and receives the frame.
//!
//! Faulted devices are retried with a fixed back-off; a device that keeps
//! failing never blocks the others.

use crate::state;
use freergb_core::{Canvas, ColorCorrection, Region};
use freergb_device_api::{DeviceId, DeviceInfo, DeviceStatus, RgbDevice};
use freergb_effects::{Effect, EffectSpec};
use serde::Serialize;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const RETRY_INTERVAL: Duration = Duration::from_secs(2);
const CANVAS_W: usize = 320;
const CANVAS_H: usize = 200;

struct Managed {
    dev: Box<dyn RgbDevice>,
    region: Region,
    correction: ColorCorrection,
    last_error: Option<String>,
    next_retry: Option<Instant>,
    frames: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceSummary {
    pub info: DeviceInfo,
    pub status: DeviceStatus,
    pub leds: usize,
    pub frames: u64,
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
    devices: Vec<Managed>,
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
}

impl Engine {
    pub fn new(spec: EffectSpec, target_fps: u32) -> Self {
        let now = Instant::now();
        Self {
            devices: Vec::new(),
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
        }
    }

    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }

    pub fn has_device(&self, id: &DeviceId) -> bool {
        self.devices.iter().any(|m| &m.dev.info().id == id)
    }

    /// Adds a device and tries to connect it immediately. A failed connect is
    /// recorded and retried by `tick`.
    pub fn add_device(&mut self, mut dev: Box<dyn RgbDevice>) {
        let (last_error, next_retry) = match dev.connect() {
            Ok(()) => (None, None),
            Err(e) => {
                tracing::warn!(device = %dev.info().id, error = %e, "connect failed");
                (Some(e.to_string()), Some(Instant::now() + RETRY_INTERVAL))
            }
        };
        tracing::info!(device = %dev.info().id, name = %dev.info().name, "device added");
        self.devices.push(Managed {
            dev,
            region: Region::FULL,
            correction: ColorCorrection::default(),
            last_error,
            next_retry,
            frames: 0,
        });
    }

    /// Restores effect and brightness from `path` and saves every later change
    /// there.
    pub fn with_state_file(mut self, path: PathBuf) -> Self {
        let s = state::load(&path);
        self.set_effect(s.effect);
        self.brightness = s.brightness.clamp(0.0, 1.0);
        self.state_path = Some(path);
        self
    }

    fn persist(&self) {
        if let Some(path) = &self.state_path {
            let s = state::State {
                effect: self.spec.clone(),
                brightness: self.brightness,
            };
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

    pub fn set_brightness(&mut self, b: f32) {
        self.brightness = b.clamp(0.0, 1.0);
        self.persist();
    }

    /// Renders one frame at `now` and pushes it to all connected devices.
    pub fn tick(&mut self, now: Instant) {
        self.effect.render(
            now.saturating_duration_since(self.effect_start),
            &mut self.canvas,
        );
        let brightness = self.brightness;

        for m in &mut self.devices {
            match m.dev.status() {
                DeviceStatus::Connected => {
                    let frame = self.canvas.sample_layout(m.dev.layout(), m.region);
                    let corr = ColorCorrection {
                        brightness: m.correction.brightness * brightness,
                        ..m.correction
                    };
                    match m.dev.push_frame(&frame.corrected(&corr)) {
                        Ok(()) => m.frames += 1,
                        Err(e) => {
                            tracing::warn!(device = %m.dev.info().id, error = %e, "frame failed");
                            m.last_error = Some(e.to_string());
                            m.next_retry = Some(now + RETRY_INTERVAL);
                        }
                    }
                }
                DeviceStatus::Disconnected | DeviceStatus::Faulted(_) => {
                    if m.next_retry.is_some_and(|t| now >= t) {
                        let _ = m.dev.disconnect(None);
                        match m.dev.connect() {
                            Ok(()) => {
                                tracing::info!(device = %m.dev.info().id, "reconnected");
                                m.last_error = None;
                                m.next_retry = None;
                            }
                            Err(e) => {
                                m.last_error = Some(e.to_string());
                                m.next_retry = Some(now + RETRY_INTERVAL);
                            }
                        }
                    }
                }
            }
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
        self.devices
            .iter()
            .map(|m| DeviceSummary {
                info: m.dev.info().clone(),
                status: m.dev.status(),
                leds: m.dev.layout().len(),
                frames: m.frames,
                last_error: m.last_error.clone(),
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
            devices: self.devices.len(),
        }
    }

    /// Releases every device back to hardware mode.
    pub fn shutdown(&mut self) {
        for m in &mut self.devices {
            if let Err(e) = m.dev.disconnect(None) {
                tracing::warn!(device = %m.dev.info().id, error = %e, "disconnect failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freergb_core::{LedLayout, Rgb};
    use freergb_device_api::{DeviceError, VirtualDevice};

    #[test]
    fn renders_to_virtual_device_with_brightness() {
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::WHITE }, 30);
        let dev = VirtualDevice::new("v", LedLayout::strip(4));
        let log = dev.frame_log();
        e.add_device(Box::new(dev));
        e.tick(Instant::now());
        assert_eq!(log.last().unwrap().colors(), &[Rgb::WHITE; 4]);

        e.set_brightness(0.5);
        e.set_effect(EffectSpec::Solid { color: Rgb::RED });
        e.tick(Instant::now());
        assert_eq!(log.last().unwrap().colors(), &[Rgb::new(128, 0, 0); 4]);
        assert_eq!(e.devices()[0].frames, 2);
        e.shutdown();
        assert_eq!(e.devices()[0].status, DeviceStatus::Disconnected);
    }

    /// Fails the first `fail_connects` connects and every push while
    /// `fail_push` is set.
    struct Flaky {
        inner: VirtualDevice,
        fail_connects: u32,
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
        fn push_frame(&mut self, f: &freergb_core::Frame) -> Result<(), DeviceError> {
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
        let mut e = Engine::new(EffectSpec::Solid { color: Rgb::BLUE }, 30);
        let inner = VirtualDevice::new("flaky", LedLayout::strip(2));
        let log = inner.frame_log();
        e.add_device(Box::new(Flaky {
            inner,
            fail_connects: 1,
        }));
        let t0 = Instant::now();
        e.tick(t0);
        assert!(e.devices()[0].last_error.is_some());
        assert_eq!(log.total(), 0);

        e.tick(t0 + RETRY_INTERVAL); // reconnects
        e.tick(t0 + RETRY_INTERVAL + Duration::from_millis(33)); // pushes
        assert_eq!(e.devices()[0].status, DeviceStatus::Connected);
        assert_eq!(e.devices()[0].last_error, None);
        assert_eq!(log.total(), 1);
    }
}
