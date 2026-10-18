//! System-audio capture.
//!
//! * Windows: WASAPI loopback of the default output device (what you hear).
//! * Linux: the default input device. To react to playback, make the
//!   "Monitor of …" source the default (e.g. `pactl set-default-source
//!   <sink>.monitor`) or pass its name. Not yet tested on Linux.
//!
//! The capture thread owns the platform stream, keeps the newest samples in a
//! small ring buffer and publishes [`AudioFeatures`] about every 15 ms.

use crate::analysis::{Analyzer, AudioFeatures};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const FFT_SIZE: usize = 2048;
const UPDATE: Duration = Duration::from_millis(15);

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("no audio device: {0}")]
    NoDevice(String),
    #[error("audio backend: {0}")]
    Backend(String),
}

/// Running audio capture. Dropping it stops the capture thread.
pub struct AudioInput {
    features: Arc<Mutex<AudioFeatures>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    device_name: String,
}

impl AudioInput {
    /// Starts capturing what the system plays (Windows) or the default input
    /// (Linux, see module docs).
    pub fn start_system_audio() -> Result<Self, CaptureError> {
        let features = Arc::new(Mutex::new(AudioFeatures::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<String, CaptureError>>();

        let (f, s) = (features.clone(), stop.clone());
        let handle = std::thread::Builder::new()
            .name("audio-capture".into())
            .spawn(move || run(f, s, ready_tx))
            .map_err(|e| CaptureError::Backend(e.to_string()))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(device_name)) => Ok(Self {
                features,
                stop,
                handle: Some(handle),
                device_name,
            }),
            Ok(Err(e)) => {
                let _ = handle.join();
                Err(e)
            }
            Err(_) => {
                stop.store(true, Ordering::SeqCst);
                Err(CaptureError::Backend("audio thread did not start".into()))
            }
        }
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Latest analysis result.
    pub fn features(&self) -> AudioFeatures {
        *self.features.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Shared handle for consumers (e.g. effects) that outlive a borrow.
    pub fn shared(&self) -> Arc<Mutex<AudioFeatures>> {
        self.features.clone()
    }
}

impl Drop for AudioInput {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn pick_device() -> Result<cpal::Device, CaptureError> {
    let host = cpal::default_host();
    // On WASAPI an input stream on an *output* device is a loopback capture.
    #[cfg(windows)]
    let dev = host.default_output_device();
    #[cfg(not(windows))]
    let dev = host.default_input_device();
    dev.ok_or_else(|| CaptureError::NoDevice("no default audio device".into()))
}

/// Everything the capture thread keeps alive.
struct Running {
    stream: cpal::Stream,
    name: String,
    rate: u32,
    ring: Arc<Mutex<VecDeque<f32>>>,
}

fn run(
    features: Arc<Mutex<AudioFeatures>>,
    stop: Arc<AtomicBool>,
    ready: mpsc::Sender<Result<String, CaptureError>>,
) {
    let setup = || -> Result<Running, CaptureError> {
        let device = pick_device()?;
        #[allow(deprecated)]
        let name = device.name().unwrap_or_else(|_| "unknown device".into());
        #[cfg(windows)]
        let config = device.default_output_config();
        #[cfg(not(windows))]
        let config = device.default_input_config();
        let config = config.map_err(|e| CaptureError::Backend(e.to_string()))?;
        let channels = usize::from(config.channels()).max(1);
        let rate = config.sample_rate();
        let ring = Arc::new(Mutex::new(VecDeque::with_capacity(FFT_SIZE * 2)));

        let push = {
            let ring = ring.clone();
            move |mono: &mut dyn Iterator<Item = f32>| {
                let mut r = ring.lock().unwrap_or_else(|e| e.into_inner());
                for s in mono {
                    if r.len() == FFT_SIZE * 2 {
                        r.pop_front();
                    }
                    r.push_back(s);
                }
            }
        };
        let err_fn = |e| tracing::warn!(error = %e, "audio stream error");
        let stream_config: cpal::StreamConfig = config.clone().into();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    push(
                        &mut data
                            .chunks(channels)
                            .map(|c| c.iter().sum::<f32>() / channels as f32),
                    );
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    push(&mut data.chunks(channels).map(|c| {
                        c.iter().map(|&s| f32::from(s) / 32768.0).sum::<f32>() / channels as f32
                    }));
                },
                err_fn,
                None,
            ),
            other => {
                return Err(CaptureError::Backend(format!(
                    "unsupported sample format {other:?}"
                )))
            }
        }
        .map_err(|e| CaptureError::Backend(e.to_string()))?;
        stream
            .play()
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        Ok(Running {
            stream,
            name,
            rate,
            ring,
        })
    };

    let Running {
        stream,
        name,
        rate,
        ring,
    } = match setup() {
        Ok(v) => v,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    tracing::info!(device = %name, sample_rate = rate, "audio capture started");
    let _ = ready.send(Ok(name));

    let mut analyzer = Analyzer::new(rate, FFT_SIZE);
    let mut window = Vec::with_capacity(FFT_SIZE);
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(UPDATE);
        window.clear();
        window.extend(
            ring.lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .copied(),
        );
        let f = analyzer.process(&window);
        *features.lock().unwrap_or_else(|e| e.into_inner()) = f;
    }
    drop(stream);
    tracing::info!("audio capture stopped");
}
