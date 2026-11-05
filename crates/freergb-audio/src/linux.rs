//! PulseAudio protocol works with both PulseAudio and pipewire-pulse.
//! Pure decoding/selection is also compiled in Windows fixture tests.

fn monitor_source(sink: &str) -> Result<String, String> {
    let sink = sink.trim();
    if sink.is_empty() || sink.len() > 1024 || sink.chars().any(char::is_control) {
        return Err("No valid default playback sink from pactl".into());
    }
    Ok(format!("{sink}.monitor"))
}

#[derive(Default)]
struct Decoder {
    pending: Vec<u8>,
}
impl Decoder {
    // Stereo float32 LE, even when pipe reads split a sample or stereo pair.
    fn push(&mut self, bytes: &[u8]) -> Vec<f32> {
        self.pending.extend_from_slice(bytes);
        let complete = self.pending.len() / 8 * 8;
        let samples = self.pending[..complete]
            .as_chunks::<8>()
            .0
            .iter()
            .map(|pair| {
                let left = f32::from_le_bytes(pair[..4].try_into().unwrap());
                let right = f32::from_le_bytes(pair[4..].try_into().unwrap());
                if left.is_finite() && right.is_finite() {
                    (left.clamp(-1.0, 1.0) + right.clamp(-1.0, 1.0)) * 0.5
                } else {
                    0.0
                }
            })
            .collect();
        self.pending.drain(..complete);
        samples
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn run(
    features: std::sync::Arc<std::sync::Mutex<crate::AudioFeatures>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ready: std::sync::mpsc::Sender<Result<String, crate::CaptureError>>,
) {
    use crate::{Analyzer, CaptureError};
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::sync::{atomic::Ordering, mpsc};
    use std::time::{Duration, Instant};

    let outcome = (|| -> Result<(), CaptureError> {
        let mut query = Command::new("pactl")
            .arg("get-default-sink")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                CaptureError::Backend(format!("pactl (pulseaudio-utils) required: {e}"))
            })?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let result = loop {
            match query.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline && !stop.load(Ordering::SeqCst) => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                _ => {
                    let _ = query.kill();
                    let _ = query.wait();
                    return Err(CaptureError::Backend(
                        "pactl unavailable or timed out".into(),
                    ));
                }
            }
        };
        if !result.success() {
            return Err(CaptureError::NoDevice(
                "No PulseAudio/PipeWire playback sink".into(),
            ));
        }
        let mut sink = String::new();
        query
            .stdout
            .take()
            .unwrap()
            .take(1026)
            .read_to_string(&mut sink)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        let source = monitor_source(&sink).map_err(CaptureError::NoDevice)?;
        let mut child = Command::new("parec")
            .args([
                "--raw",
                "--format=float32le",
                "--rate=48000",
                "--channels=2",
            ])
            .arg(format!("--device={source}"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                CaptureError::Backend(format!("parec (pulseaudio-utils) required: {e}"))
            })?;
        let mut output = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::sync_channel(4);
        let reader = std::thread::spawn(move || {
            let mut bytes = [0u8; 8192];
            while let Ok(n) = output.read(&mut bytes) {
                if n == 0 {
                    break;
                }
                // Backpressure retains byte alignment; kill closes the pipe on stop.
                if tx.send(bytes[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        let mut decoder = Decoder::default();
        let mut window = std::collections::VecDeque::with_capacity(4096);
        let mut analyzer = Analyzer::new(48000, 2048);
        let mut started = false;
        let mut last = Instant::now();
        while !stop.load(Ordering::SeqCst) {
            match rx.recv_timeout(Duration::from_millis(15)) {
                Ok(bytes) => {
                    last = Instant::now();
                    for sample in decoder.push(&bytes) {
                        if window.len() == 4096 {
                            window.pop_front();
                        }
                        window.push_back(sample);
                    }
                    if !started && !window.is_empty() {
                        started = true;
                        let _ = ready.send(Ok(source.clone()));
                    }
                    *features.lock().unwrap_or_else(|e| e.into_inner()) =
                        analyzer.process(&window.iter().copied().collect::<Vec<_>>());
                }
                Err(mpsc::RecvTimeoutError::Timeout) if last.elapsed() < Duration::from_secs(3) => {
                }
                _ => break,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        drop(rx);
        let _ = reader.join();
        *features.lock().unwrap_or_else(|e| e.into_inner()) = crate::AudioFeatures::default();
        if !started {
            return Err(CaptureError::Backend(
                "Playback monitor produced no samples".into(),
            ));
        }
        Ok(())
    })();
    if let Err(error) = outcome {
        let _ = ready.send(Err(error));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_playback_monitor_without_changing_default_source() {
        assert_eq!(
            monitor_source("speaker.output\n").unwrap(),
            "speaker.output.monitor"
        );
        for bad in ["", "\n", "one\ntwo", "bad\0source"] {
            assert!(monitor_source(bad).is_err());
        }
        assert!(monitor_source(&"x".repeat(1025)).is_err());
    }
    #[test]
    fn fragmented_pcm_is_bounded_sanitized_and_downmixed() {
        let bytes: Vec<_> = [1.0f32, -0.5, f32::NAN, 1.0, 4.0, 4.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        let mut decoder = Decoder::default();
        let samples: Vec<_> = bytes.chunks(3).flat_map(|b| decoder.push(b)).collect();
        assert_eq!(samples, [0.25, 0.0, 1.0]);
        assert!(decoder.pending.is_empty());
    }
}
