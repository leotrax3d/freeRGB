//! Screen capture for ambience effects.
//!
//! A background thread captures the primary monitor (via `xcap`), box-filters
//! it down to a small [`ScreenFrame`] and publishes it. Only the downscaled
//! frame leaves the thread; nothing is stored or sent anywhere. The daemon
//! runs the capture only while a screen effect is active.

use freergb_core::ScreenFrame;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_os = "linux"))]
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
#[cfg(not(target_os = "linux"))]
use std::time::{Duration, Instant};

#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::run_helper as run_linux_helper;

/// Resolution of the published frame (16:9).
pub const FRAME_W: usize = 64;
pub const FRAME_H: usize = 36;

pub type ScreenHandle = Arc<Mutex<ScreenFrame>>;

#[derive(Debug, thiserror::Error)]
pub enum ScreenError {
    #[error("no monitor found")]
    NoMonitor,
    #[error("screen capture: {0}")]
    Capture(String),
}

/// Running screen capture. Dropping it stops the thread.
pub struct ScreenInput {
    frame: ScreenHandle,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    monitor_name: String,
    #[cfg(target_os = "linux")]
    capture_state: Arc<std::sync::atomic::AtomicU8>,
}

#[cfg(not(target_os = "linux"))]
fn primary_monitor() -> Result<xcap::Monitor, ScreenError> {
    let monitors = xcap::Monitor::all().map_err(|e| ScreenError::Capture(e.to_string()))?;
    let mut fallback = None;
    for m in monitors {
        if m.is_primary().unwrap_or(false) {
            return Ok(m);
        }
        fallback.get_or_insert(m);
    }
    fallback.ok_or(ScreenError::NoMonitor)
}

impl ScreenInput {
    /// Captures the primary monitor at up to `fps` frames per second.
    pub fn start_primary(fps: u32) -> Result<Self, ScreenError> {
        #[cfg(target_os = "linux")]
        {
            linux::start(fps)
        }
        #[cfg(not(target_os = "linux"))]
        Self::start_native(fps)
    }

    #[cfg(not(target_os = "linux"))]
    fn start_native(fps: u32) -> Result<Self, ScreenError> {
        let frame: ScreenHandle = Arc::new(Mutex::new(ScreenFrame::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<String, ScreenError>>();
        let period = Duration::from_secs_f64(1.0 / f64::from(fps.clamp(1, 60)));

        let (f, s) = (frame.clone(), stop.clone());
        let handle = std::thread::Builder::new()
            .name("screen-capture".into())
            .spawn(move || {
                let monitor = match primary_monitor() {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let name = monitor.name().unwrap_or_else(|_| "monitor".into());
                // First capture decides whether capturing works at all.
                if let Err(e) = monitor.capture_image() {
                    let _ = ready_tx.send(Err(ScreenError::Capture(e.to_string())));
                    return;
                }
                tracing::info!(monitor = %name, "screen capture started");
                let _ = ready_tx.send(Ok(name));
                let mut failures = 0u32;
                while !s.load(Ordering::SeqCst) {
                    let t0 = Instant::now();
                    match monitor.capture_image() {
                        Ok(img) => {
                            failures = 0;
                            let (w, h) = (img.width() as usize, img.height() as usize);
                            let small =
                                ScreenFrame::downscale_rgba(img.as_raw(), w, h, FRAME_W, FRAME_H);
                            *f.lock().unwrap_or_else(|e| e.into_inner()) = small;
                        }
                        Err(e) => {
                            failures += 1;
                            if failures == 1 || failures.is_multiple_of(100) {
                                tracing::warn!(error = %e, failures, "screen capture failed");
                            }
                        }
                    }
                    if let Some(rest) = period.checked_sub(t0.elapsed()) {
                        std::thread::sleep(rest);
                    }
                }
                tracing::info!("screen capture stopped");
            })
            .map_err(|e| ScreenError::Capture(e.to_string()))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(monitor_name)) => Ok(Self {
                frame,
                stop,
                handle: Some(handle),
                monitor_name,
            }),
            Ok(Err(e)) => {
                let _ = handle.join();
                Err(e)
            }
            Err(_) => {
                stop.store(true, Ordering::SeqCst);
                Err(ScreenError::Capture("capture thread did not start".into()))
            }
        }
    }

    pub fn monitor_name(&self) -> &str {
        #[cfg(target_os = "linux")]
        match self.capture_state.load(Ordering::SeqCst) {
            0 => return "Waiting for screen capture / portal consent (up to 60s)",
            2 => return "Screen capture denied, unavailable or stopped; reselect effect to retry",
            _ => {}
        }
        &self.monitor_name
    }

    pub fn frame(&self) -> ScreenFrame {
        self.frame.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn shared(&self) -> ScreenHandle {
        self.frame.clone()
    }
}

impl Drop for ScreenInput {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
